# Koffice v0.8 — MCP-сервер и Python SDK

В проект добавлены **отдельный MCP-сервер на Rust** и **Python SDK без сторонних зависимостей**. Они вызывают общие операции Writer, Sheets и Slides, как и настольный Automation API.

**Ограничение:** MCP работает со своей изолированной сессией в памяти. Он не подключается к открытым вкладкам Koffice Desktop, не обращается к файлам пользователя и не может управлять Koffice Base.

## Сборка и тестирование

```sh
cargo build --manifest-path src-tauri/Cargo.toml --bin koffice-mcp
cargo test --manifest-path src-tauri/Cargo.toml
python -m unittest discover -s python -p 'test_*.py' -v
```

Результат сборки: `src-tauri/target/debug/koffice-mcp` (Windows: `koffice-mcp.exe`). В настройках MCP-клиента добавьте новый сервер с транспортом **stdio** и командой, указывающей на этот исполняемый файл.

Сервер поддерживает MCP JSON-RPC и три инструмента:

- `koffice_snapshot`: состояние открытых в сессии документов и их ревизии.
- `koffice_execute`: пакет из 1–50 операций (при ошибке отменяется весь пакет).
- `koffice_reset`: сброс временной сессии.

Python SDK находится в `python/koffice_mcp.py`. Пример использования — `python -m koffice_mcp` при доступном исполняемом файле в PATH, или посмотрите документацию `docs/MCP.md`.

Сервер не разрешает произвольные пути, процессы, сетевые запросы и сохранение документов. Состояние теряется при завершении процесса. Сборка и тесты проверяются GitHub Actions, включая Python-тесты взаимодействия с настоящим сервером.

Полная поддержка форматов Microsoft Office, доступ к открытым документам и интеграция Base через MCP остаются следующими задачами.
