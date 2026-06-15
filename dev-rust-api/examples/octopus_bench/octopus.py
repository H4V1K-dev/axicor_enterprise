#!/usr/bin/env python3
"""
Сценарий инициализации OctopusBrain модели.
Используется для демонстрации и нагрузочного тестирования линтера и структуры Python SDK.
Связи (connections) и сокеты временно исключены для фокуса на базовой структуре.
"""

from axipy import ModelBuilder, Shard, Department

def build_octopus_brain():
    model = ModelBuilder(project_name="OctopusBrain", output_dir="./octopus_recipe")

    # Загружаем заготовки типов нейронов
    # В реальном коде это будут TOML-файлы библиотек. Для мока считаем, что они доступны.
    photoreceptor = model.gnm_lib("bio/sensory/photoreceptor")
    hair_cell     = model.gnm_lib("bio/sensory/hair_cell")
    pyramidal     = model.gnm_lib("bio/cortex/pyramidal_exc")
    basket_inh    = model.gnm_lib("bio/cortex/basket_inh")
    purkinje      = model.gnm_lib("bio/motor/purkinje")
    motor_neuron  = model.gnm_lib("bio/motor/spinal_motor")

    # =====================================================================
    # ДЕПАРТАМЕНТ 1: SensoryInput (Первичные сенсоры и реле)
    # =====================================================================
    sensory_dept = Department("SensoryInput")

    # Шард 1: Retina
    retina = Shard("Retina", x=128, y=128, z=10)
    retina.add_layer("L1_Sensory", height_pct=1.0, density=0.85)
    retina.add_population("L1_Sensory", photoreceptor, fraction=1.0)
    # Входы
    retina.add_input_port("camera_L", width=32, height=32)
    retina.add_input_port("camera_R", width=32, height=32)
    # Сокеты
    retina.add_socket("optic_nerve", width=16, height=16)
    retina.add_socket("sync_out", width=4, height=4)
    retina.add_socket("sync_in", width=4, height=4)
    retina.add_socket("fast_bypass", width=8, height=8)

    # Шард 2: Cochlea
    cochlea = Shard("Cochlea", x=64, y=64, z=5)
    cochlea.add_layer("L1_Sensory", height_pct=1.0, density=0.9)
    cochlea.add_population("L1_Sensory", hair_cell, fraction=1.0)
    # Входы
    cochlea.add_input_port("mic_low", width=8, height=8)
    cochlea.add_input_port("mic_mid", width=8, height=8)
    cochlea.add_input_port("mic_high", width=8, height=8)
    # Сокеты
    cochlea.add_socket("auditory_nerve", width=8, height=8)
    cochlea.add_socket("sync_out", width=4, height=4)
    cochlea.add_socket("sync_in", width=4, height=4)
    cochlea.add_socket("fast_bypass", width=4, height=4)

    # Шард 3: Thalamus
    thalamus = Shard("Thalamus", x=64, y=64, z=20)
    thalamus.add_layer("Relay", height_pct=0.8, density=0.6)
    thalamus.add_population("Relay", pyramidal, fraction=1.0)
    thalamus.add_layer("Feedback_Gates", height_pct=0.2, density=0.5)
    thalamus.add_population("Feedback_Gates", basket_inh, fraction=1.0)
    # Сокеты
    thalamus.add_socket("visual_input", width=16, height=16)
    thalamus.add_socket("auditory_input", width=8, height=8)
    thalamus.add_socket("feedback_in", width=8, height=8)
    thalamus.add_socket("top_down_gate", width=4, height=4)
    thalamus.add_socket("visual_thalamocortical", width=16, height=16)
    thalamus.add_socket("auditory_thalamocortical", width=8, height=8)

    sensory_dept.add_shard(retina, cochlea, thalamus)

    # =====================================================================
    # ДЕПАРТАМЕНТ 2: CentralProcessor (Кора, интеграция)
    # =====================================================================
    # Инициализация департамента CentralProcessor
    central_dept = Department("CentralProcessor")

    # Шард 1: VisualCortex
    v_cortex = Shard("VisualCortex", x=256, y=256, z=40)
    v_cortex.add_layer("L4_Input", height_pct=0.5, density=0.7)
    v_cortex.add_population("L4_Input", pyramidal, fraction=1.0)
    v_cortex.add_layer("L2_3_Feedback", height_pct=0.5, density=0.5)
    v_cortex.add_population("L2_3_Feedback", basket_inh, fraction=1.0)
    # Входы
    v_cortex.add_input_port("camera_L_bypass", width=16, height=16)
    # Сокеты
    v_cortex.add_socket("thalamic_in", width=16, height=16)
    v_cortex.add_socket("feedback_in", width=8, height=8)
    v_cortex.add_socket("x_modal_in", width=8, height=8)
    v_cortex.add_socket("assoc_fb_in", width=8, height=8)
    v_cortex.add_socket("fast_bypass_in", width=8, height=8)
    v_cortex.add_socket("cortex_out", width=16, height=16)
    v_cortex.add_socket("x_modal_out", width=8, height=8)

    # Шард 2: AuditoryCortex
    a_cortex = Shard("AuditoryCortex", x=128, y=128, z=40)
    a_cortex.add_layer("L4_Input", height_pct=1.0, density=0.7)
    a_cortex.add_population("L4_Input", pyramidal, fraction=1.0)
    # Входы
    a_cortex.add_input_port("direct_high_freq", width=8, height=8)
    # Сокеты
    a_cortex.add_socket("thalamic_in", width=8, height=8)
    a_cortex.add_socket("x_modal_in", width=8, height=8)
    a_cortex.add_socket("fast_bypass_in", width=4, height=4)
    a_cortex.add_socket("cortex_out", width=8, height=8)
    a_cortex.add_socket("x_modal_out", width=8, height=8)
    a_cortex.add_socket("direct_aud_cmd", width=6, height=6)

    # Шард 3: AssociationArea
    assoc_area = Shard("AssociationArea", x=128, y=128, z=30)
    assoc_area.add_layer("Integration", height_pct=1.0, density=0.8)
    assoc_area.add_population("Integration", pyramidal, fraction=0.8)
    assoc_area.add_population("Integration", basket_inh, fraction=0.2)
    # Сокеты
    assoc_area.add_socket("visual_in", width=16, height=16)
    assoc_area.add_socket("auditory_in", width=8, height=8)
    assoc_area.add_socket("integrated_out", width=16, height=16)
    assoc_area.add_socket("assoc_fb_out", width=8, height=8)
    assoc_area.add_socket("direct_motor_cmd", width=8, height=8)

    # Шард 4: Prefrontal
    prefrontal = Shard("Prefrontal", x=64, y=64, z=50)
    prefrontal.add_layer("Decision", height_pct=1.0, density=0.9)
    prefrontal.add_population("Decision", pyramidal, fraction=1.0)
    # Выходы
    prefrontal.add_output_port("head_orient", width=2, height=2)
    # Сокеты
    prefrontal.add_socket("sensory_in", width=16, height=16)
    prefrontal.add_socket("direct_aud_cmd_in", width=6, height=6)
    prefrontal.add_socket("attention_feedback", width=8, height=8)
    prefrontal.add_socket("motor_commands", width=12, height=12)
    prefrontal.add_socket("top_down_gate", width=4, height=4)

    # Добавляем все шарды в департамент CentralProcessor
    central_dept.add_shard(v_cortex, a_cortex, assoc_area, prefrontal)

    # =====================================================================
    # ДЕПАРТАМЕНТ 3: MotorControl (Двигательные ядра и эффекторы)
    # =====================================================================
    motor_dept = Department("MotorControl")

    # Шард 1: MotorCortex
    m_cortex = Shard("MotorCortex", x=128, y=128, z=30)
    m_cortex.add_layer("L5_Output", height_pct=1.0, density=0.6)
    m_cortex.add_population("L5_Output", pyramidal, fraction=1.0)
    # Сокеты
    m_cortex.add_socket("command_in", width=12, height=12)
    m_cortex.add_socket("direct_motor_cmd_in", width=8, height=8)
    m_cortex.add_socket("cerebellar_fb_in", width=8, height=8)
    m_cortex.add_socket("cortex_to_spinal", width=8, height=8)
    m_cortex.add_socket("cortex_to_cerebellum", width=8, height=8)

    # Шард 2: Cerebellum
    cerebellum = Shard("Cerebellum", x=256, y=256, z=15)
    cerebellum.add_layer("Purkinje_Layer", height_pct=1.0, density=0.9)
    cerebellum.add_population("Purkinje_Layer", purkinje, fraction=1.0)
    # Выходы
    cerebellum.add_output_port("muscle_LB", width=4, height=4)
    cerebellum.add_output_port("muscle_RB", width=4, height=4)
    # Сокеты
    cerebellum.add_socket("plan_in", width=8, height=8)
    cerebellum.add_socket("local_proprioception_in", width=4, height=4)
    cerebellum.add_socket("correction_out", width=8, height=8)
    cerebellum.add_socket("proprioceptive_feedback", width=8, height=8)
    cerebellum.add_socket("cerebellar_fb", width=8, height=8)

    # Шард 3: SpinalCord
    spinal = Shard("SpinalCord", x=32, y=32, z=60)
    spinal.add_layer("MotorPool", height_pct=1.0, density=0.4)
    spinal.add_population("MotorPool", motor_neuron, fraction=1.0)
    # Выходы
    spinal.add_output_port("muscle_LF", width=4, height=4)
    spinal.add_output_port("muscle_RF", width=4, height=4)
    # Сокеты
    spinal.add_socket("descending_in", width=8, height=8)
    spinal.add_socket("coordination_in", width=8, height=8)
    spinal.add_socket("local_proprioception_out", width=4, height=4)

    motor_dept.add_shard(m_cortex, cerebellum, spinal)

    # =====================================================================
    # Коммутация связей
    # =====================================================================
    
    # 1. SensoryInput (Внутренние связи)
    retina.sockets["optic_nerve"].connect_to("Thalamus.visual_input")
    cochlea.sockets["auditory_nerve"].connect_to("Thalamus.auditory_input")
    retina.sockets["sync_out"].connect_to("Cochlea.sync_in")
    cochlea.sockets["sync_out"].connect_to("Retina.sync_in")
    
    # 2. CentralProcessor (Внутренние связи)
    v_cortex.sockets["cortex_out"].connect_to("AssociationArea.visual_in")
    a_cortex.sockets["cortex_out"].connect_to("AssociationArea.auditory_in")
    assoc_area.sockets["integrated_out"].connect_to("Prefrontal.sensory_in")
    prefrontal.sockets["attention_feedback"].connect_to("VisualCortex.feedback_in")
    v_cortex.sockets["x_modal_out"].connect_to("AuditoryCortex.x_modal_in")
    a_cortex.sockets["x_modal_out"].connect_to("VisualCortex.x_modal_in")
    a_cortex.sockets["direct_aud_cmd"].connect_to("Prefrontal.direct_aud_cmd_in")
    assoc_area.sockets["assoc_fb_out"].connect_to("VisualCortex.assoc_fb_in")
    
    # 3. MotorControl (Внутренние связи)
    m_cortex.sockets["cortex_to_spinal"].connect_to("SpinalCord.descending_in")
    m_cortex.sockets["cortex_to_cerebellum"].connect_to("Cerebellum.plan_in")
    cerebellum.sockets["correction_out"].connect_to("SpinalCord.coordination_in")
    cerebellum.sockets["cerebellar_fb"].connect_to("MotorCortex.cerebellar_fb_in")
    spinal.sockets["local_proprioception_out"].connect_to("Cerebellum.local_proprioception_in")
    
    # 4. Междепартаментные связи
    thalamus.sockets["visual_thalamocortical"].connect_to("CentralProcessor.VisualCortex.thalamic_in")
    thalamus.sockets["auditory_thalamocortical"].connect_to("CentralProcessor.AuditoryCortex.thalamic_in")
    retina.sockets["fast_bypass"].connect_to("CentralProcessor.VisualCortex.fast_bypass_in")
    cochlea.sockets["fast_bypass"].connect_to("CentralProcessor.AuditoryCortex.fast_bypass_in")
    prefrontal.sockets["motor_commands"].connect_to("MotorControl.MotorCortex.command_in")
    prefrontal.sockets["top_down_gate"].connect_to("SensoryInput.Thalamus.top_down_gate")
    assoc_area.sockets["direct_motor_cmd"].connect_to("MotorControl.MotorCortex.direct_motor_cmd_in")
    cerebellum.sockets["proprioceptive_feedback"].connect_to("SensoryInput.Thalamus.feedback_in")

    # =====================================================================
    # РЕГИСТРАЦИЯ В БИЛДЕРЕ
    # =====================================================================
    model.add_department(sensory_dept, central_dept, motor_dept)

    # Запускаем отладочную сборку (будет фейлиться, пока не пропишем линковку,
    # но проверит базовые ограничения)
    model.build(dry_run=True)

if __name__ == "__main__":
    build_octopus_brain()
