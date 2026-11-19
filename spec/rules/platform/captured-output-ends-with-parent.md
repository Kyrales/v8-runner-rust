---
id: INV.PLATFORM.CAPTURED-OUTPUT-ENDS-WITH-PARENT
check:
  - src/platform/process.rs::captured_parent_exit_does_not_wait_for_descendant
  - src/platform/process.rs::snapshot_is_a_fixed_prefix_and_preserves_writer_position
  - src/platform/process.rs::capture_short_read_is_reported
  - src/platform/process.rs::capture_read_failure_is_reported
  - src/platform/process.rs::captured_streams_preserve_large_bytes_and_lossy_logs
---

# Захват вывода заканчивается снимком после выхода основного процесса

Обычный `ProcessExecutor::run_with_policy` после подтверждённого выхода основного
процесса читает конечные префиксы stdout/stderr. Потомок с унаследованными stdio
не продлевает ожидание. Код выхода и политика прерывания основного процесса сохраняются;
обычный выход сам по себе не завершает потомков. Живой критический процесс по-прежнему
дожидается безопасного завершения.

Единственный владелец захвата — `CapturedOutput` в `platform/process.rs`: безымянные
временные файлы с независимыми reader/writer handles; границы stdout/stderr фиксируются
перед чтением. Поздние записи потомков не входят в обещанный снимок. Ошибки чтения
не превращаются в пустой успешный вывод. Blocking readers, оставленные после возврата
потоки и ожидание EOF от потомков нельзя возвращать под другим именем.

Windows `seek_read` изменяет reader cursor, поэтому `try_clone` недостаточно для
отделения reader от writer: используется `NamedTempFile::reopen`. Имя удаляется с
проверкой результата до spawn; OS освобождает хранилище после последнего handle.
