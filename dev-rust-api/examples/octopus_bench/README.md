# Octopus Brain Benchmark & DSL Test

Этот проект служит полигоном для тестирования возможностей Python SDK (линтера, структуры, сборщика) на сложной топологии.

## Шаг 1: Инициализация структуры (Модель ==> Департаменты ==> Шарды)

На первом этапе мы создаем базовый скелет модели без связей, сокетов и внешних портов:
1. **Инициализация `ModelBuilder`**: Создание модели `OctopusBrain`.
2. **Инициализация Департаментов**: 
   * `SensoryInput` (сенсоры)
   * `CentralProcessor` (кора)
   * `MotorControl` (движение)
3. **Инициализация Шардов**: Создание вычислительных блоков (Retina, Cochlea, Thalamus, VisualCortex, AuditoryCortex, AssociationArea, Prefrontal, MotorCortex, Cerebellum, SpinalCord), настройка их геометрических размеров и анатомических слоев.
4. **Загрузка Нейронных Блупринтов**: Импорт физических параметров клеток из библиотек (`gnm_lib`).

## Шаг 2: Интерфейсы (Порты ввода-вывода и Сокеты)

На втором этапе мы добавляем интерфейсы взаимодействия для каждого шарда (без прокладки связей между ними):
1. **Внешние порты (`add_input_port` / `add_output_port`)**: Логические интерфейсы для общения с внешним миром (камерами, микрофонами, двигателями робота).
2. **Внутренние сокеты (`add_socket`)**: Интерфейсы для прорастания аксонов (как входящие, так и исходящие).

На этом этапе все сокеты висят свободными, ожидая дальнейшей коммутации.

## Шаг 3: Коммутация связей (Линковка)

На третьем этапе мы соединяем сокеты между собой, выстраивая проводку внутри департаментов и между ними:
1. **Внутренние связи (`connect_to`)**: Связи между сокетами внутри одного департамента. Адресация ведется по принципу `"TargetShard.TargetSocket"`.
2. **Междепартаментные связи**: Связи между сокетами разных департаментов. Адресация/разрешение имен ведется по полному пути `"Department.TargetShard.TargetSocket"`.

Линтер SDK автоматически проверяет корректность типов сокетов (чтобы передатчик соединялся с приемником) и совпадение их размерностей.

---

## Схема связей модели (Целевой граф)

```mermaid
flowchart TD

  %% ═══════════════════════════════
  %% DEPT 1: SensoryInput
  %% ═══════════════════════════════
  subgraph SensoryInput [Департамент: SensoryInput]
    Retina["Retina\nPhotoSensor"]
    Cochlea["Cochlea\nFreqResonance"]
    Thalamus["Thalamus\nRelay gate"]

    Retina -->|optic_nerve| Thalamus
    Cochlea -->|auditory_nerve| Thalamus
    Retina -.->|sync| Cochlea
    Cochlea -.->|sync| Retina
  end

  %% ═══════════════════════════════
  %% DEPT 2: CentralProcessor
  %% ═══════════════════════════════
  subgraph CentralProcessor [Департамент: CentralProcessor]
    VisualCortex["VisualCortex\nV1–V4 hierarchy"]
    AuditoryCortex["AuditoryCortex\nTonotopic A1/A2"]
    AssociationArea["AssociationArea\nMultimodal hub"]
    Prefrontal["Prefrontal\nExecutive WM"]

    VisualCortex -->|cortex_out| AssociationArea
    AuditoryCortex -->|cortex_out| AssociationArea
    AssociationArea -->|integrated_out| Prefrontal
    Prefrontal -.->|attention_feedback ↻| VisualCortex
    VisualCortex -.->|x-modal| AuditoryCortex
    AuditoryCortex -.->|x-modal| VisualCortex
    AuditoryCortex -.->|direct_aud_cmd| Prefrontal
    AssociationArea -.->|assoc_fb ↻| VisualCortex
  end

  %% ═══════════════════════════════
  %% DEPT 3: MotorControl
  %% ═══════════════════════════════
  subgraph MotorControl [Департамент: MotorControl]
    MotorCortex["MotorCortex\nTopomap"]
    Cerebellum["Cerebellum\nError-correction"]
    SpinalCord["SpinalCord\nMN-pool"]

    MotorCortex -->|cortex_to_spinal| SpinalCord
    MotorCortex -->|cortex_to_cerebellum| Cerebellum
    Cerebellum -->|correction_out| SpinalCord
    SpinalCord -.->|local_proprioception ↻| Cerebellum
    Cerebellum -.->|cerebellar_fb ↻| MotorCortex
  end

  %% ═══════════════════════════════
  %% CROSS-DEPARTMENT
  %% ═══════════════════════════════
  Thalamus ==>|visual_thalamocortical| VisualCortex
  Thalamus ==>|auditory_thalamocortical| AuditoryCortex
  Prefrontal ==>|motor_commands| MotorCortex
  Cerebellum -.->|proprioceptive_feedback ↻| Thalamus
  AssociationArea -.->|direct_motor_cmd| MotorCortex
  Prefrontal -.->|top-down gate| Thalamus
  Retina -.->|fast bypass| VisualCortex
  Cochlea -.->|fast bypass| AuditoryCortex

  %% ═══════════════════════════════
  %% EXTERNAL I/O
  %% ═══════════════════════════════
  CamL([Camera L]) ==>|/in: camera_L| Retina
  CamR([Camera R]) ==>|/in: camera_R| Retina
  CamL -.->|fast bypass| VisualCortex

  MicLow([Mic Low]) ==>|/in: mic_low| Cochlea
  MicMid([Mic Mid]) ==>|/in: mic_mid| Cochlea
  MicHigh([Mic High]) ==>|/in: mic_high| Cochlea
  MicHigh -.->|direct high-freq| AuditoryCortex

  SpinalCord ==>|/out: muscle_LF| LF([LF])
  SpinalCord ==>|/out: muscle_RF| RF([RF])
  Cerebellum ==>|/out: muscle_LB| LB([LB])
  Cerebellum ==>|/out: muscle_RB| RB([RB])
  Prefrontal ==>|/out: head_orient| Head([Head])
```