#!/usr/bin/env python3
import os
import sys

# =====================================================================
# КЛАССЫ СДК С ДВУСТОРОННИМИ СОКЕТАМИ, ЛИНТЕРОМ И ГЕНЕРАЦИЕЙ ПАСПОРТА
# =====================================================================

class NeuronBlueprint:
    def __init__(self, name: str):
        self.name = name

class Layer:
    def __init__(self, name: str, height_pct: float, density: float):
        self.name = name
        self.height_pct = height_pct
        self.density = density
        self.populations = []

class Socket:
    def __init__(self, name: str, shard: 'Shard', width: int, height: int, entry_z: str = None):
        self.name = name
        self.shard = shard
        self.width = width
        self.height = height
        self.entry_z = entry_z  # Задается только для входящих сокетов
        self.connections = []    # Куда ведет этот сокет (если исходящий)
        self.connected_by = None # Откуда прилетела связь (если входящий)

    def connect_to(self, target_path: str) -> 'Socket':
        """Соединяет этот исходящий сокет с входящим сокетом другого шарда."""
        self.connections.append(target_path)
        return self


class Shard:
    def __init__(self, name: str, x: int, y: int, z: int):
        self.name = name
        self.x = x
        self.y = y
        self.z = z
        self.layers = {}
        self.input_ports = []
        self.output_ports = []
        self.sockets = {}
        self.department_name = None

    def add_layer(self, name: str, height_pct: float, density: float) -> 'Shard':
        self.layers[name] = Layer(name, height_pct, density)
        return self

    def add_population(self, layer_name: str, blueprint: NeuronBlueprint, fraction: float) -> 'Shard':
        if layer_name not in self.layers:
            raise ValueError(f"Слой '{layer_name}' не найден в шарде '{self.name}'!")
        self.layers[layer_name].populations.append((blueprint, fraction))
        return self

    def add_input_port(self, name: str, width: int, height: int, entry_z: str, target_type: str = "All", growth_steps: int = 1000) -> 'Shard':
        self.input_ports.append({
            "name": name, "width": width, "height": height, 
            "entry_z": entry_z, "target_type": target_type, "growth_steps": growth_steps
        })
        return self

    def add_output_port(self, name: str, width: int, height: int) -> 'Shard':
        self.output_ports.append({
            "name": name, "width": width, "height": height
        })
        return self

    def add_socket(self, name: str, width: int, height: int, entry_z: str = None) -> Socket:
        socket = Socket(name, self, width, height, entry_z)
        self.sockets[name] = socket
        return socket

    def estimate_neurons(self) -> int:
        """Предварительный расчет количества нейронов на основе размеров и слоев."""
        total_vol = self.x * self.y * self.z
        estimated = 0
        for layer in self.layers.values():
            # Объем слоя = объем шарда * доля высоты слоя
            layer_vol = total_vol * layer.height_pct
            estimated += int(layer_vol * layer.density)
        return estimated


class Department:
    def __init__(self, name: str):
        self.name = name
        self.shards = {}

    def add_shard(self, *shards: Shard) -> 'Department':
        for shard in shards:
            shard.department_name = self.name
            self.shards[shard.name] = shard
        return self


class ModelBuilder:
    def __init__(self, project_name: str, output_dir: str):
        self.project_name = project_name
        self.output_dir = output_dir
        self.departments = {}

    def add_department(self, *depts: Department) -> 'ModelBuilder':
        for dept in depts:
            self.departments[dept.name] = dept
        return self

    def gnm_lib(self, path: str) -> NeuronBlueprint:
        return NeuronBlueprint(path.split("/")[-1])

    def _find_shard_and_socket(self, path: str) -> (Shard, Socket):
        try:
            shard_name, socket_name = path.split(".")
        except ValueError:
            raise ValueError(f"Неверный формат пути сокета: '{path}'. Ожидалось 'ИмяШарда.ИмяСокета'")
        
        for dept in self.departments.values():
            if shard_name in dept.shards:
                shard = dept.shards[shard_name]
                if socket_name in shard.sockets:
                    return shard, shard.sockets[socket_name]
        return None, None

    def build(self, dry_run: bool = False, print_level: str = "all") -> 'ModelBuilder':
        # 1. Валидация
        internal_connections = {}
        global_connections = []
        
        # Линтер связей
        for dept_name, dept in self.departments.items():
            internal_connections[dept_name] = []
            for shard_name, shard in dept.shards.items():
                for socket_name, socket in shard.sockets.items():
                    if socket.entry_z is not None:
                        continue
                    for target_path in socket.connections:
                        target_shard, target_socket = self._find_shard_and_socket(target_path)
                        if not target_shard or not target_socket:
                            raise ValueError(f"❌ ОШИБКА: Целевой сокет '{target_path}' не найден!")
                        if target_socket.entry_z is None:
                            raise ValueError(f"❌ ОШИБКА: Сокет '{target_path}' является исходящим!")
                        if socket.width != target_socket.width or socket.height != target_socket.height:
                            raise ValueError(f"❌ ОШИБКА РАЗМЕРНОСТЕЙ: {shard_name}.{socket_name} -> {target_path}")
                        if target_socket.connected_by is not None:
                            raise ValueError(f"❌ ОШИБКА: Сокет '{target_path}' уже занят!")
                        
                        target_socket.connected_by = f"{shard_name}.{socket_name}"
                        resolved_z = target_socket.entry_z
                        if resolved_z in target_shard.layers:
                            resolved_z = f"layer_center({resolved_z})"

                        connection_data = {
                            "from_shard": shard_name,
                            "socket": socket_name,
                            "to_shard": target_shard.name,
                            "target_socket": target_socket.name,
                            "width": socket.width,
                            "height": socket.height,
                            "entry_z": target_socket.entry_z,
                            "resolved_z": resolved_z
                        }

                        if target_shard.department_name == dept_name:
                            internal_connections[dept_name].append(connection_data)
                        else:
                            connection_data["from_dept"] = dept_name
                            connection_data["to_dept"] = target_shard.department_name
                            global_connections.append(connection_data)

        # Вывод в консоль в зависимости от print_level
        if print_level in ("all", "console"):
            print(f"\n=== СБОРКА МОДЕЛИ: {self.project_name} ===")
            print(f"Статус: {'DRY RUN' if dry_run else 'PRODUCTION BUILD'}")
            print(f"Выходная директория: {self.output_dir}\n")
            
            if print_level == "all":
                print("1. Структура:")
                for dept_name, dept in self.departments.items():
                    print(f"  🏢 Департамент: '{dept_name}'")
                    for shard_name, shard in dept.shards.items():
                        est_n = shard.estimate_neurons()
                        print(f"    🧠 Шард: '{shard_name}' [Размеры: {shard.x}x{shard.y}x{shard.z}, Оценка нейронов: {est_n}]")
                
                print("\n2. Локальные связи:")
                for dept_name, conns in internal_connections.items():
                    print(f"  🏢 Внутри '{dept_name}':")
                    for c in conns:
                        print(f"    🔗 {c['from_shard']}.{c['socket']} -> {c['to_shard']}.{c['target_socket']}")
                
                print("\n3. Глобальные связи:")
                for c in global_connections:
                    print(f"  🌐 {c['from_dept']}({c['from_shard']}) -> {c['to_dept']}({c['to_shard']})")

        # Генерация паспорта
        self.generate_passport(internal_connections, global_connections)
        
        if not dry_run:
            # Здесь в реальной жизни мы бы записали TOML файлы в output_dir
            pass

        return self

    def generate_passport(self, internal_connections: dict, global_connections: list):
        """Создает файл паспорта модели (model_pass.md) в папке проекта."""
        os.makedirs(self.output_dir, exist_ok=True)
        passport_path = os.path.join(self.output_dir, "model_pass.md")
        
        # Считаем базовые ТТХ
        total_neurons = 0
        total_shards = 0
        for dept in self.departments.values():
            for shard in dept.shards.values():
                total_shards += 1
                total_neurons += shard.estimate_neurons()
                
        # Расчет VRAM (приблизительный: 1166 байт на нейрон + 32 байта на аксон связи)
        est_vram_mb = (total_neurons * 1166) / (1024 * 1024)

        with open(passport_path, "w", encoding="utf-8") as f:
            f.write(f"# Паспорт спецификации модели: {self.project_name}\n\n")
            
            f.write("## 📊 Тактико-технические характеристики (ТТХ)\n")
            f.write(f"* **Количество департаментов**: {len(self.departments)}\n")
            f.write(f"* **Количество шардов (зон)**: {total_shards}\n")
            f.write(f"* **Общее расчетное число нейронов**: {total_neurons:,} шт.\n")
            f.write(f"* **Ориентировочный бюджет VRAM**: {est_vram_mb:.2f} MB\n\n")
            
            f.write("## 🏢 Логическая архитектура\n")
            f.write("| Департамент | Шард | Физический объем | Слои | Популяции | Внешние Порты |\n")
            f.write("|---|---|---|---|---|---|\n")
            for dept_name, dept in self.departments.items():
                for shard_name, shard in dept.shards.items():
                    layers_str = "<br>".join(shard.layers.keys())
                    
                    pops = []
                    for layer in shard.layers.values():
                        for bp, frac in layer.populations:
                            pops.append(f"{bp.name} ({frac*100:.0f}%)")
                    pops_str = "<br>".join(pops)
                    
                    ports = []
                    for p in shard.input_ports:
                        ports.append(f"IN: {p['name']}")
                    for p in shard.output_ports:
                        ports.append(f"OUT: {p['name']}")
                    ports_str = "<br>".join(ports) if ports else "Нет"
                    
                    f.write(f"| `{dept_name}` | `{shard_name}` | {shard.x}x{shard.y}x{shard.z} | {layers_str} | {pops_str} | {ports_str} |\n")
            
            f.write("\n## 🔗 Граф связей (Mermaid)\n")
            f.write("```mermaid\n")
            f.write("flowchart TD\n")
            
            # Отрисовка департаментов и их шардов
            for dept_name, dept in self.departments.items():
                f.write(f"  subgraph {dept_name} [\"Департамент: {dept_name}\"]\n")
                for shard_name in dept.shards.keys():
                    safe_id = shard_name.replace("-", "_")
                    f.write(f"    {safe_id}[\"Шард: {shard_name}\"]\n")
                f.write("  end\n")
            
            # Отрисовка локальных связей
            for dept_name, conns in internal_connections.items():
                for c in conns:
                    from_safe = c['from_shard'].replace("-", "_")
                    to_safe = c['to_shard'].replace("-", "_")
                    label = f"сокет: {c['socket']} (Z={c['entry_z']})"
                    f.write(f"  {from_safe} -->|\"{label}\"| {to_safe}\n")
            
            # Отрисовка глобальных связей
            for c in global_connections:
                from_safe = c['from_shard'].replace("-", "_")
                to_safe = c['to_shard'].replace("-", "_")
                label = f"межотдельский тракт: {c['socket']}"
                f.write(f"  {from_safe} ==>|\"{label}\"| {to_safe}\n")
                
            f.write("```\n")
            
        print(f"📖 Паспорт модели успешно сгенерирован: {passport_path}")

    def bake(self, filename: str) -> 'ModelBuilder':
        print(f"AOT-компиляция завершена -> {filename}")
        return self


# =====================================================================
# ДЕМОНСТРАЦИОННЫЙ СЦЕНАРИЙ (Fish Model Connectome)
# =====================================================================

def build_fish_brain():
    # Инициализируем модель с указанием выходной папки рецепта
    model = ModelBuilder(project_name="FishBrainConnectome", output_dir="./fish_recipe")

    # Берем чертежи нейронов из библиотеки
    photoreceptor = model.gnm_lib("Sensory/Photoreceptor")
    relay_neuron  = model.gnm_lib("Cortex/L4/spiny/VISp4/1")
    fast_spiking  = model.gnm_lib("Cortex/L4/aspiny/VISp4/1")
    pacemaker     = model.gnm_lib("Thalamus/Pacemaker")

    # ============================================================
    # ШАРД 1: (Retina)
    # ============================================================
    retina = Shard("Retina", x=32, y=32, z=8)
    retina.add_layer("Photoreceptors", height_pct=1.0, density=0.9)
    retina.add_population("Photoreceptors", photoreceptor, fraction=1.0)
    retina.add_input_port("retinal_image", width=16, height=16, entry_z="top")
    
    # Исходящий сокет (Передатчик)
    retina.add_socket("optic_nerve", width=8, height=8)\
          .connect_to("OpticTectum.optic_input")

    # ============================================================
    # ШАРД 2: (Optic Tectum)
    # ============================================================
    tectum = Shard("OpticTectum", x=64, y=64, z=32)
    tectum.add_layer("StratumOpticum", height_pct=0.4, density=0.5)
    tectum.add_population("StratumOpticum", relay_neuron, fraction=1.0)
    tectum.add_layer("DeepLayers", height_pct=0.6, density=0.7)
    tectum.add_population("DeepLayers", fast_spiking, fraction=1.0)
    
    # Входящий сокет (Приемник)
    tectum.add_socket("optic_input", width=8, height=8, entry_z="StratumOpticum")
    
    # Исходящий сокет (Передатчик)
    tectum.add_socket("tectal_efferent", width=8, height=8)\
          .connect_to("LocomotorCPG.descending_input")

    # ============================================================
    # ШАРД 3: (Locomotor CPG)
    # ============================================================
    cpg = Shard("LocomotorCPG", x=16, y=16, z=16)
    cpg.add_layer("CPG_Core", height_pct=1.0, density=0.3)
    cpg.add_population("CPG_Core", pacemaker, fraction=1.0)
    cpg.add_output_port("fins_drive", width=4, height=4)
    cpg.add_socket("descending_input", width=8, height=8, entry_z="top")

    # ============================================================
    # СБОРКА СТРУКТУРЫ МОДЕЛИ
    # ============================================================
    
    # Собираем департаменты
    vis_dept = Department("visual_system").add_shard(retina, tectum)
    mot_dept = Department("motor_system").add_shard(cpg)

    # Регистрируем в модели
    model.add_department(vis_dept, mot_dept)

    # Запускаем сборку в режиме DRY RUN (отладка без записи файлов)
    # Линтер проверит все связи, рассчитает ТТХ и запишет model_pass.md
    model.build(dry_run=True, print_level="all").bake("fish_brain.axic")


if __name__ == "__main__":
    build_fish_brain()
