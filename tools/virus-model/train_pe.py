"""Train a local PE risk model from the licensed EMBER2018 feature archive.

Requires numpy, scikit-learn, and onnx. Reads labeled feature records only.
The upstream test split is reserved for the final evaluation.
"""

import argparse
import hashlib
import json
import math
import tarfile
from pathlib import Path

import numpy as np
import onnx
from onnx import TensorProto, helper, numpy_helper

ARCHIVE_HASH = "b6052eb8d350a49a8d5a5396fbe7d16cf42848b86ff969b77464434cf2997812"
HEADER_FIELDS = (
    "major_linker_version", "minor_linker_version", "major_image_version",
    "minor_image_version", "major_operating_system_version", "minor_operating_system_version",
    "major_subsystem_version", "minor_subsystem_version", "sizeof_code", "sizeof_headers",
    "sizeof_heap_commit",
)
FEATURE_COUNT = 1 + 4 + len(HEADER_FIELDS)


def feature_row(record):
    header = record["header"]
    optional = header["optional"]
    if optional["magic"] not in ("PE32", "PE32_PLUS"):
        return None
    values = [record["general"]["size"], *(optional[name] for name in HEADER_FIELDS)]
    if any(not isinstance(value, int) or value < 0 for value in values):
        return None
    return [
        math.log1p(values[0]),
        float(optional["magic"] == "PE32_PLUS"),
        float(optional["subsystem"] == "WINDOWS_GUI"),
        float(optional["subsystem"] == "WINDOWS_CUI"),
        float("DLL" in header["coff"]["characteristics"]),
        *(math.log1p(value) for value in values[1:]),
    ]


def read_splits(archive):
    with archive.open("rb") as source:
        if hashlib.file_digest(source, "sha256").hexdigest() != ARCHIVE_HASH:
            raise ValueError("unexpected EMBER archive SHA-256")
    splits = {name: ([], []) for name in ("train", "validation", "test")}
    with tarfile.open(archive, "r|bz2") as bundle:
        for entry in bundle:
            name = Path(entry.name).name
            is_test = name == "test_features.jsonl"
            if not entry.isfile() or not (is_test or name.startswith("train_features_")):
                continue
            with bundle.extractfile(entry) as rows:
                for line in rows:
                    record = json.loads(line)
                    label = record["label"]
                    if label not in (0, 1):
                        continue
                    pick = int(record["sha256"][:8], 16) % 10
                    if not is_test and pick > 3:
                        continue
                    features = feature_row(record)
                    if features is None:
                        continue
                    split = "test" if is_test else "validation" if pick == 0 else "train"
                    x, y = splits[split]
                    x.append(features)
                    y.append(label)
            print(f"Read {name}", flush=True)
    if any(set(labels) != {0, 1} for _, labels in splits.values()):
        raise ValueError("each split must contain both labels")
    return {
        name: (np.asarray(x, dtype=np.float32), np.asarray(y, dtype=np.bool_))
        for name, (x, y) in splits.items()
    }


def rates(scores, labels, threshold):
    benign = scores[~labels]
    malicious = scores[labels]
    return {
        "benign": len(benign),
        "malicious": len(malicious),
        "false_positive": float(np.mean(benign >= threshold)),
        "recall": float(np.mean(malicious >= threshold)),
    }


def export_network(network, scaler):
    # Fold the training scaler into the first affine layer.
    first = network.coefs_[0].astype(np.float64) / scaler.scale_[:, None]
    bias = network.intercepts_[0] - scaler.mean_ @ first
    tensors = {
        "first": first,
        "bias": bias,
        "output_weights": network.coefs_[1],
        "output_bias": network.intercepts_[1],
    }
    graph = helper.make_graph(
        [
            helper.make_node("Gemm", ["features", "first", "bias"], ["hidden"]),
            helper.make_node("Relu", ["hidden"], ["activated"]),
            helper.make_node("Gemm", ["activated", "output_weights", "output_bias"], ["logit"]),
            helper.make_node("Sigmoid", ["logit"], ["risk"]),
        ],
        "ochistka_pe_header_v1",
        [helper.make_tensor_value_info("features", TensorProto.FLOAT, [1, FEATURE_COUNT])],
        [helper.make_tensor_value_info("risk", TensorProto.FLOAT, [1, 1])],
        [numpy_helper.from_array(np.asarray(value, dtype=np.float32), name) for name, value in tensors.items()],
    )
    return helper.make_model(graph, producer_name="OCHISTKA", opset_imports=[helper.make_opsetid("", 13)], ir_version=8)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("archive", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    splits = read_splits(args.archive)
    from sklearn.neural_network import MLPClassifier
    from sklearn.preprocessing import StandardScaler

    train, train_labels = splits["train"]
    scaler = StandardScaler().fit(train)
    network = MLPClassifier(hidden_layer_sizes=(32,), batch_size=512, max_iter=35, random_state=193)
    network.fit(scaler.transform(train), train_labels)
    validation, validation_labels = splits["validation"]
    val_scores = network.predict_proba(scaler.transform(validation))[:, 1]
    cutoff = np.float32(np.quantile(val_scores[~validation_labels], 0.997, method="higher"))
    threshold = float(np.nextafter(cutoff, np.float32(1.0)))
    test, test_labels = splits["test"]
    test_scores = network.predict_proba(scaler.transform(test))[:, 1]
    metadata = {
        "dataset_sha256": ARCHIVE_HASH,
        "training_samples": len(train),
        "threshold": threshold,
        "validation": rates(val_scores, validation_labels, threshold),
        "test": rates(test_scores, test_labels, threshold),
    }
    print(json.dumps(metadata, indent=2), flush=True)
    args.output.mkdir(parents=True, exist_ok=True)
    # A model can be useful as a suspicion signal even at low recall. Keep false positives bounded.
    if metadata["test"]["false_positive"] > 0.01 or metadata["test"]["recall"] < 0.15:
        (args.output / "rejected.json").write_text(json.dumps(metadata, indent=2) + "\n", encoding="utf-8")
        raise SystemExit("held-out quality thresholds were not met")
    model = export_network(network, scaler)
    onnx.checker.check_model(model)
    from onnx.reference import ReferenceEvaluator

    evaluator = ReferenceEvaluator(model)
    for row, expected in zip(test[:12], test_scores[:12], strict=True):
        actual = evaluator.run(None, {"features": row[None, :]})[0].item()
        if abs(actual - expected) > 0.0001:
            raise ValueError("ONNX output differs from the trained model")
    model_path = args.output / "pe-header-v1.onnx"
    onnx.save(model, model_path)
    metadata["model_sha256"] = hashlib.sha256(model_path.read_bytes()).hexdigest()
    (args.output / "pe-header-v1.json").write_text(json.dumps(metadata, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
