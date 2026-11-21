#!/usr/bin/env python3
"""Простой запуск диагностики OCHISTKA для Windows x64 (без Python и без ручных команд).

Это ТОЛЬКО добровольная проверка одного сайта. Она не включает «Обход блокировок» в приложении.
Все ограничения безопасности берутся из external_dpi_probe.py: проверенные SHA-256 файлов, действующая
подпись драйвера, перехват одного соединения, тайм-ауты, обязательная остановка и очистка.
Права администратора НЕ запрашиваются автоматически: окно просит запустить файл от имени
администратора самому пользователю. Защита Windows не отключается.
"""

from __future__ import annotations

import datetime
import io
import pathlib
import socket
import sys

import external_dpi_probe as probe

TITLE = "OCHISTKA: проверка обхода блокировки одного сайта"
SELF_CHECK = "--self-check"

VERDICT_TEXT = {
    "inconclusive_baseline_already_reachable": "Сайт и так открывается: проверять нечего, движок не запускался.",
    "no_confirmed_improvement": "Улучшения не замечено: с проверяемой стратегией сайт не открылся.",
    "inconclusive_control_changed": "Результат неоднозначный: контрольные замеры отличаются. Нужен повтор.",
    "repeatable_connectivity_improvement_not_proof_of_dpi": (
        "Замечено улучшение доступа. Это ещё не доказательство обхода блокировки "
        "и не готовый режим для приложения."
    ),
}


def configure_unicode_output() -> None:
    """Windows CI pipes may default to cp1252, which cannot print Russian prompts.

    A standalone Windows executable must render its own messages reliably,
    without relying on an external chcp or PYTHONIOENCODING setting.
    """
    for stream in (sys.stdout, sys.stderr):
        reconfigure = getattr(stream, "reconfigure", None)
        if callable(reconfigure):
            reconfigure(encoding="utf-8", errors="replace")


def app_dir() -> pathlib.Path:
    if getattr(sys, "frozen", False):
        return pathlib.Path(sys.executable).resolve().parent
    return pathlib.Path(__file__).resolve().parent


def bundle_dir() -> pathlib.Path:
    return app_dir() / "bundle"


def self_check(folder: pathlib.Path | None = None, lua_dir: pathlib.Path | None = None) -> None:
    """Офлайн-проверка набора: хэши пяти файлов и двух Lua. Сеть и драйвер не используются."""
    probe.verify_release_files(folder if folder is not None else bundle_dir())
    probe.verify_lua_files(lua_dir if lua_dir is not None else probe.lua_source_dir())


def resolve_ipv4(host: str) -> list[str]:
    """Публичные IPv4 сайта по системному DNS. Подлинность адреса проверяет сертификат TLS."""
    name, _ = probe.validated_target(host, "1.1.1.1")  # проверка имени (адрес-заглушка публичный)
    found: list[str] = []
    for item in socket.getaddrinfo(name, probe.PORT, socket.AF_INET, socket.SOCK_STREAM):
        address = item[4][0]
        try:
            probe.validated_target(name, address)
        except ValueError:
            continue
        if address not in found:
            found.append(address)
    return found


def ask_yes(question: str, ask=input) -> bool:
    """Согласие только словом ДА (регистр не важен); всё остальное — отказ."""
    return ask(f"{question}\nНапишите ДА, чтобы согласиться, или нажмите Enter, чтобы отказаться: ").strip().upper() == "ДА"


class Tee(io.TextIOBase):
    """Дублирует вывод в файл отчёта. В отчёт попадают только названия исходов, время и имя сайта."""

    def __init__(self, console, report) -> None:
        self.console, self.report = console, report

    def write(self, text: str) -> int:
        self.console.write(text)
        self.report.write(text)
        return len(text)

    def flush(self) -> None:
        self.console.flush()
        self.report.flush()


def run_interactive(ask=input, folder: pathlib.Path | None = None) -> int:
    configure_unicode_output()
    print(TITLE)
    print("=" * len(TITLE))
    if sys.platform != "win32":
        print("Эта проверка работает только в Windows 10/11 x64.")
        return 1
    if not probe.is_admin_windows():
        print("Нужны права администратора.")
        print("Закройте окно, нажмите на файл правой кнопкой мыши и выберите «Запуск от имени администратора».")
        print("Программа сама права не запрашивает и защиту Windows не меняет.")
        return 1
    try:
        self_check(folder)
    except (RuntimeError, OSError) as exc:
        print(f"Набор файлов не прошёл проверку, запуск остановлен: {exc}")
        return 1
    print("Что делает проверка:")
    print(" - один раз открывает указанный вами сайт БЕЗ обхода и запоминает результат;")
    print(" - если сайт не открывается, на несколько секунд включает проверяемый движок,")
    print("   который затрагивает ТОЛЬКО это одно тестовое соединение к этому сайту;")
    print(" - выключает движок, открывает сайт ещё раз и показывает итог;")
    print(" - ничего не меняет в настройках Windows, DNS, прокси и защите.")
    print()
    host = ask("Введите адрес сайта без https://, например example.org: ").strip().lower()
    try:
        addresses = resolve_ipv4(host)
    except (ValueError, OSError) as exc:
        print(f"Не удалось подготовить проверку для этого адреса: {exc}")
        return 1
    if not addresses:
        print("У сайта не найдено подходящего публичного адреса IPv4.")
        return 1
    address = addresses[0]
    print(f"Будет проверен сайт {host}, адрес {address}, порт 443.")
    if not ask_yes("Согласны на кратковременный перехват этого одного соединения?", ask):
        print("Отказ принят. Ничего не запускалось.")
        return 0
    if not ask_yes("VPN и другие прокси-программы выключены?", ask):
        print("Сначала выключите VPN и повторите проверку. Ничего не запускалось.")
        return 0
    stamp = datetime.datetime.now().strftime("%Y%m%d-%H%M%S")
    report_path = app_dir() / f"OCHISTKA-DPI-report-{stamp}.txt"
    code = 1
    with report_path.open("w", encoding="utf-8") as report:
        real_out = sys.stdout
        sys.stdout = Tee(real_out, report)
        try:
            print(f"Сайт: {host}; время: {stamp}")
            code = probe.main([
                "--host", host, "--ip", address,
                "--bundle-dir", str(folder if folder is not None else bundle_dir()),
                "--execute", "--acknowledge-narrow-interception", "--confirm-vpn-off",
            ])
        except (RuntimeError, ValueError, OSError) as exc:
            print(f"ПРОВЕРКА ПРЕРВАНА: {exc}")
            code = 1
        finally:
            sys.stdout = real_out
    print()
    print(f"Отчёт сохранён рядом с программой: {report_path.name}")
    print("В отчёте нет ссылок, паролей и содержимого страниц.")
    return code


def main(argv: list[str] | None = None) -> int:
    configure_unicode_output()
    args = sys.argv[1:] if argv is None else argv
    if args == [SELF_CHECK]:
        try:
            self_check()
        except (RuntimeError, OSError) as exc:
            print(f"SELF-CHECK FAILED: {exc}")
            return 1
        print("SELF-CHECK OK: хэши файлов движка и Lua совпали (сеть и драйвер не использовались)")
        return 0
    if args:
        print("Запускайте без параметров (двойной щелчок).")
        return 2
    try:
        code = run_interactive()
    finally:
        if sys.stdin and sys.stdin.isatty():
            input("\nНажмите Enter, чтобы закрыть окно...")
    return code


if __name__ == "__main__":
    raise SystemExit(main())
