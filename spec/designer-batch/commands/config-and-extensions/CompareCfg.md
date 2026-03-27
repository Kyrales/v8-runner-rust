# CompareCfg

- Вид: `command`
- Раздел: Конфигурация и расширения
- Путь в оглавлении: Конфигурация и расширения -> CompareCfg
- Source pagePath: `zif3_comparecfg`
- Source URL: `http://localhost:8080/ru/1%D0%A1%3A%D0%9F%D1%80%D0%B5%D0%B4%D0%BF%D1%80%D0%B8%D1%8F%D1%82%D0%B8%D0%B5?page=zif3_comparecfg`

## Синтаксис

```text
/CompareCfg -FirstConfigurationType [-FirstName] [-FirstFile] [-FirstVersion]
-SecondConfigurationType [-SecondName] [-SecondFile] [-SecondVersion] [-MappingRule]
[-Objects] -ReportType [-IncludeChangedObjects] [-IncludeDeletedObjects] [-IncludeAddedObjects] -ReportFormat -ReportFile
```

## Нормализованное описание

— построение отчета о сравнении конфигурации. Доступны опции:

**-FirstConfigurationType** — тип первой конфигурации для сравнения. Возможны значения:

- **MainConfiguration** — основная конфигурация;

- **DBConfiguration** — конфигурация базы данных;

- **VendorConfiguration** — конфигурация поставщика;

- **ExtensionConfiguration** — расширение конфигурации;

- **ExtensionDBConfiguration** — расширение конфигурации (база данных);

- **ConfigurationRepository** — конфигурация из хранилища конфигурации;

- **ExtensionConfigurationRepository** — расширение конфигурации из хранилища расширения конфигурации;

- **File** — файл конфигурации/расширения конфигурации.

**-FirstName **— имя конфигурации. Зависит от типа конфигурации:

- **VendorConfiguration** — имя конфигурации поставщика;

- **ExtensionConfiguration** — имя расширения конфигурации;

- **ExtensionDBConfiguration** — имя расширения конфигурации (база данных);

**-FirstFile **— путь к файлу. Используется при указании типа конфигурации **File**.

**-FirstVersion **— версия конфигурации хранилища. Используется при указании типа конфигурации **ConfigurationRepository** и **ExtensionConfigurationRepository**. 

**-SecondConfigurationType** — тип второй конфигурации для сравнения. Возможны значения:

- **MainConfiguration** — основная конфигурация;

- **DBConfiguration** — конфигурация базы данных;

- **VendorConfiguration** — конфигурация поставщика;

- **ExtensionConfiguration** — расширение конфигурации;

- **ExtensionDBConfiguration** — расширение конфигурации (база данных);

- **ConfigurationRepository** — конфигурация из хранилища конфигурации;

- **File** — файл конфигурации/расширения конфигурации.

**-SecondName **— имя конфигурации. Зависит от типа конфигурации:

- **VendorConfiguration** — имя конфигурации поставщика;

- **ExtensionConfiguration** — имя расширения конфигурации;

- **ExtensionDBConfiguration** — имя расширения конфигурации (база данных);

**-SecondFile **— путь к файлу. Используется при указании типа конфигурации **File**.

**-SecondVersion **— версия конфигурации хранилища. Используется при указании типа конфигурации **ConfigurationRepository** и **ExtensionConfigurationRepository**. 

**-MappingRule** — правило установки соответствий объектов для тех случаев, когда конфигурации не состоят в отношениях «родитель-потомок»:. Допустимые значения:

- **ByObjectNames ** — по именам. Используется по умолчанию.

- **ByObjectIDs** — по идентификаторам.

**-Objects** — путь к файлу в формате XML, содержащему список объектов. Подробнее о формате файла см в документации. Если не указан, отчет строится по всей конфигурации.

**-ReportType** — тип отчета. Возможные значения:

- **Brief** — краткий отчет.

- **Full** — полный отчет.

**-IncludeChangedObjects** — включать в отчет измененные подчиненные объекты.

**-IncludeDeletedObjects** — включать в отчет удаленные подчиненные объекты.

**-IncludeAddedObjects** — включать в отчет добавленные подчиненные объекты.

**-ReportFormat** — формат файла отчета. Возможные значения:

- **txt** — текстовый документ.

- **mxl** — табличный документ.

**-ReportFile** — путь к результирующему файлу отчета.
