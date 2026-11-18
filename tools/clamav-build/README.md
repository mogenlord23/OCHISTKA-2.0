# Сборка libclamav

1. `./fetch.sh` — скачивает ClamAV 1.4.6 из Debian и проверяет SHA-256.
2. Зависимости для CMake (обязательные): OpenSSL, zlib, bzip2, libxml2, PCRE2, iconv. На Windows — через vcpkg.
3. Конфигурация (Linux, для проверки): `cmake -S src -B build -DENABLE_APP=OFF -DENABLE_TESTS=OFF -DENABLE_MILTER=OFF -DENABLE_CLAMONACC=OFF -DENABLE_SHARED_LIB=OFF -DENABLE_STATIC_LIB=ON -DCMAKE_BUILD_TYPE=Release`
4. Windows (MSVC) — отдельно, с vcpkg; в этой среде не выполнялось.

Статус: в среде разработки сборка остановилась на отсутствии `libxml2` и `pcre2` (системные пакеты не установлены). См. `docs/status/clamav-build.md`.
