> **Fusion v0.17:** встроенные таблицы Sheets можно свободно перемещать и изменять по размеру в Slides с помощью мыши или стрелок клавиатуры. Положение сохраняется внутри `.kslide` и при экспорте. [Инструкция](docs/SLIDES-LAYOUT.ru.md).

> **Fusion v0.16:** В Slides можно вставлять настоящие редактируемые листы Sheets с формулами, сохранять их внутри `.kslide`, возвращать обратно в Sheets и включать в печать/HTML/показ. [Инструкция](docs/SLIDES-SHEETS.ru.md). ИИ остаётся необязательным.

> **Fusion v0.15:** Base создаёт связанные слайды презентации с сохранённым источником `.kdb`, ручным обновлением и защитой изменённого вручную слайда. Связь сохраняется внутри `.kslide` при перемещении и дублировании слайда. [Руководство](docs/BASE-SLIDES-LINKS.ru.md).

> **Fusion v0.14:** Центр управления связями показывает все встроенные объекты без чтения внешних файлов. Выбранные файловые связи можно проверить, предварительно просмотреть изменения ячеек и обновить одним атомарным пакетом с повторной проверкой источников. [Инструкция](docs/FUSION-CENTER.ru.md). Фоновой синхронизации нет.

> **Fusion v0.13:** сохраняемые связи с файлами .ksheet через перезапуск Koffice, проверка/обновление/переподключение по команде, защита локальных правок. Открытие .kdoc само по себе не читает связанные файлы. [Инструкция](docs/FUSION-FILES.ru.md).

> **Fusion v0.12:** появились управляемые связи Sheets → Writer. Проверка состояния, ручное обновление, блокировка перезаписи локальных правок, явное переподключение и отсоединение без потери таблицы. Связи действуют в рамках текущей сессии, после перезапуска требуется переподключение. [Инструкция](docs/FUSION-LINKS.ru.md).

> **Fusion v0.11:** обратный перенос встроенного листа Writer → Sheets без потери формул и без удаления других листов. Base → Slides создаёт ограниченный статический снимок таблицы с предупреждением об усечении данных. [Документация](docs/FUSION-TRANSFERS.ru.md). Живой синхронизации пока нет.

> **Koffice Fusion v0.10:** В Writer можно вставлять и редактировать автономные таблицы Sheets с формулами; они сохраняются внутри .kdoc и не требуют отдельного файла. [Описание Fusion](docs/FUSION.ru.md). Это первый этап интеграции, не полная реализация связей и внешних офисных форматов.

> **Приоритет Koffice v0.9:** кроссплатформенность, производительность, удобство и надёжность офисной работы. ИИ — дополнительная возможность, а не обязательная часть. [Изменения v0.9](docs/desktop-v0.9.md).

## Новое в v0.6

Проверка целостности SQLite, резервное копирование баз без перезаписи файлов, полный CSV-экспорт таблиц до 10 000 записей, сгруппированные отчёты COUNT/SUM/AVG и исправленная версия MSI для Windows. Импорт/экспорт DOCX/XLSX/PPTX пока не реализован.

# Koffice — открытый офисный пакет на Rust + Tauri

**Koffice v0.2.0-alpha.1** — экспериментальный офисный пакет с тремя редакторами и общими операциями на Rust. Цель — обеспечить единые функции как для пользователей, так и для будущих ИИ-агентов.

## Реализовано

- **Writer:** текстовые документы, редактирование, простые текстовые маркеры оформления, сохранение `.kdoc`, экспорт TXT, печать и PDF через системный диалог.
- **Sheets:** таблица, несколько листов, редактирование ячеек, арифметические формулы, ссылки, `SUM`, сохранение `.ksheet`, CSV-экспорт активного листа.
- **Slides:** создание/правка/удаление слайдов, заметки докладчика, просмотр презентации, сохранение `.kslide`, печать через системный диалог.
- **Rust Core:** проверка входных данных, контроль ревизий, сериализация, сохранение во временный файл, модульные тесты.

Это **ранняя альфа**, а не готовый конкурент Microsoft Office. Полная поддержка DOCX/XLSX/PPTX, сложное форматирование, диаграммы, средства совместной работы, Python SDK и MCP пока отсутствуют. Не доверяйте важные оригиналы этой версии без резервных копий.

## Запуск (Windows)

Установите Rust stable, Node.js 20+, Microsoft C++ Build Tools и WebView2. В каталоге проекта:

```powershell
npm install
npm run tauri dev
```

Тесты и сборка:

```powershell
npm test
cd src-tauri
cargo test
cd ..
npm run tauri build
```

Для доработок смотрите `src-tauri/src/engine.rs`, `src-tauri/src/suite.rs`, `src-tauri/src/lib.rs`, `src/` и `CONTRIBUTING.md`.

## Принцип AI-native

Интерфейс вызывает Rust-команды, а в будущем эти же команды станут основой защищённого RPC/MCP доступа. Нельзя объявлять MCP готовым, пока нет действующего сервера, схем инструментов и проверок разрешений.

Лицензия: MIT.

## Обновление v0.3 alpha
- **Writer:** поиск и замена отдельного совпадения; текстовое форматирование при помощи разметки.
- **Sheets:** атомарная вставка диапазонов из буфера (TSV), импорт CSV на текущий лист, экспорт CSV, переименование и удаление листов, формулы `SUM`, `AVERAGE`, `COUNT`, `MIN`, `MAX`, арифметика и одиночные ссылки на ячейки.
- **Slides:** дублирование, перемещение слайдов, заметки и предварительный просмотр презентации.
- **Rust Core:** проверка ревизий и валидация пакетных операций перед сохранением.

Это ранняя версия. Поддержки DOCX/XLSX/PPTX и сервера MCP пока нет; Writer не является полноценным визуальным редактором форматированного текста. Результаты сборки и тестов публикуются в GitHub Actions.

## Koffice Base (v0.4 alpha)

SQLite-based `.kdb` databases with a native Rust API: create/open, typed schemas, multiple tables, record CRUD, text search, read-only SQL explorer, CSV export of visible results (500-row cap). Every successful insert/update/delete is saved automatically. The SQL explorer is read-only; user-provided values in CRUD operations are parameterized.

**Limitations:** no MS Access MDB/ACCDB import, no forms, reports, visual relationships or migrations yet. Keep backups. No external database connection support yet.


## Koffice Base v0.5: database power tools

- Transactional CSV import to an existing table using its exact column names (5 MB / 10,000 rows per import). Any error rolls back the whole import. CSV headers must not contain the auto-generated `id` column.
- Index management (single or multi-column, optional unique constraint), with index listing and deletion.
- Database paging in 500-row blocks and text-column search.
- **Base → Sheets** copies a 500-row database snapshot into a new unsaved workbook; exporting large datasets will require future streaming APIs.
- New Rust tests cover rollback, quoted CSV, index uniqueness, pagination, and existing CRUD paths.

These features are experimental and do not yet provide Access file compatibility or graphical relational forms. Save backup copies before using early alpha builds.

CI includes a Windows desktop build job. If successful, Windows installers are downloadable from the corresponding GitHub Actions run artifacts (not a signed release).

## v0.7: локальный Automation API (экспериментальный)

Кнопка **Automation API** открывает локальную панель выполнения команд JSON с подтверждением пользователя. Это пока **не MCP-сервер и не Python SDK**. Движок Rust выполняет операции с Writer, Sheets и Slides в одной транзакции (1–50 операций); при ошибке весь пакет откатывается. Запись в файлы и операции Koffice Base в эту транзакцию не входят.

Поддерживаются команды `writer_edit`, `sheets_put_batch`, `sheets_add_sheet`, `sheets_rename_sheet`, `slides_add`, `slides_update`, `slides_duplicate`. Для каждой нужно указывать текущую ревизию документа. Ревизии доступны в снимке состояния.

В Sheets появились поиск по книге и статистика числовых данных (сумма, минимум, максимум, среднее, число формул). Writer и Slides теперь умеют экспортировать простые автономные HTML-файлы. Полноценная совместимость с DOCX/XLSX/PPTX ещё не готова.