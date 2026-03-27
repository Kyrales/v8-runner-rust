# AgentSSHHostKey

- Вид: `parameter`
- Раздел: Команды работы в режиме агента
- Путь в оглавлении: Команды работы в режиме агента -> AgentSSHHostKey
- Source pagePath: `zif3_AgentSSHHostKey`
- Source URL: `http://localhost:8080/ru/1%D0%A1%3A%D0%9F%D1%80%D0%B5%D0%B4%D0%BF%D1%80%D0%B8%D1%8F%D1%82%D0%B8%D0%B5?page=zif3_AgentSSHHostKey`

## Синтаксис

```text
/AgentSSHHostKey <приватный ключ>
```

## Нормализованное описание

– Параметр позволяет указать путь к закрытому ключу хоста. Если данный параметр не указан, то должен быть указан параметр **/AgentSSHHostKeyAuto**.

Если не указан ни один из параметров – запуск в режиме агента будет невозможен.
