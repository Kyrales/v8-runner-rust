# AgentSSHHostKeyAuto

- Вид: `parameter`
- Раздел: Команды работы в режиме агента
- Путь в оглавлении: Команды работы в режиме агента -> AgentSSHHostKeyAuto
- Source pagePath: `zif3_AgentSSHHostKeyAuto`
- Source URL: `http://localhost:8080/ru/1%D0%A1%3A%D0%9F%D1%80%D0%B5%D0%B4%D0%BF%D1%80%D0%B8%D1%8F%D1%82%D0%B8%D0%B5?page=zif3_AgentSSHHostKeyAuto`

## Синтаксис

```text
/AgentSSHHostKeyAuto
```

## Нормализованное описание

– указывает, что закрытый ключ хоста имеет следующее расположение (в зависимости от используемой операционной системы):

- Для ОС Windows: %LOCALAPPDATA%\1C\1cv8\host_id.

- Для ОС Linux: ~/.1cv8/1C/1cv8/host_id.

- Для ОС macOS: ~/.1cv8/1C/1cv8/host_id.

Если указанный файл не будет обнаружен, то будет создан закрытый ключ для алгоритма RSA с длиной ключа 2 048 бит.
