import urllib.request
import json
import csv
import os
import sys

specimen_ids = [
    313861608, 313862134, 491029832, 517974394, 475549334,
    490376252, 490944352, 486754703, 313860745, 313861411,
    469801138, 314900022, 321906005, 471141261, 324257146,
    325941643, 324493977
]

def fetch_json(url):
    try:
        req = urllib.request.Request(url, headers={'User-Agent': 'AxiEngine-Harvester/1.0'})
        with urllib.request.urlopen(req, timeout=10) as r:
            body = r.read().decode()
            data = json.loads(body)
            if data.get("success"):
                return data.get("msg", [])
    except Exception as e:
        print(f"  Error fetching {url}: {e}", file=sys.stderr)
    return []

def main():
    print("Starting Allen Reference Harvest for 17 specimens (Hardened v1)...")
    records = []
    
    for sid in specimen_ids:
        print(f"Processing Specimen ID: {sid}...")
        record = {
            "specimen_id": sid,
            "status": "not_found_or_failed",
            "structure_area": "n/a",
            "structure_layer": "n/a",
            "transgenic_line": "n/a",
            "full_genotype": "n/a",
            "dendrite_type": "n/a",
            "reported_cell_class": "n/a",
            "resting_membrane_potential_mv": "n/a",
            "input_resistance_mohm": "n/a",
            "tau_ms": "n/a",
            "rheobase_pa": "n/a",
            "threshold_voltage_mv": "n/a",
            "firing_rate_hz": "n/a",
            "adaptation_index": "n/a",
            "upstroke_downstroke_ratio": "n/a",
            "ap_half_width": "pending_nwb_extraction",
            "nwb_availability": False,
            "glif_model_count": 0,
            "biophysical_model_count": 0,
            "spontaneous_activity_status": "pending",
            "notes": ""
        }
        
        # 1. Fetch ApiCellTypesSpecimenDetail
        url_detail = f"http://api.brain-map.org/api/v2/data/query.json?q=model::ApiCellTypesSpecimenDetail,rma::criteria,[specimen__id$eq{sid}]"
        detail_msg = fetch_json(url_detail)
        
        # 2. Fetch EphysFeature
        url_ephys = f"http://api.brain-map.org/api/v2/data/query.json?criteria=model::EphysFeature,[specimen_id$eq{sid}]"
        ephys_msg = fetch_json(url_ephys)
        
        # Fallback to Specimen query if detail is missing
        if not detail_msg:
            url_specimen = f"http://api.brain-map.org/api/v2/data/query.json?q=model::Specimen,rma::criteria,[id$eq{sid}],rma::include,structure,donor(transgenic_lines)"
            spec_msg = fetch_json(url_specimen)
            if spec_msg:
                spec = spec_msg[0]
                record["structure_area"] = spec.get("structure", {}).get("acronym", "n/a")
                # Try to parse layer from name or acronym
                layer_id = spec.get("cortex_layer_id")
                if layer_id:
                    record["structure_layer"] = str(layer_id)
                record["full_genotype"] = spec.get("donor", {}).get("full_genotype", "n/a")
                record["notes"] += "Fetched from fallback Specimen query; detail view missing. "
                record["status"] = "partial_api_data"
        
        # Process ApiCellTypesSpecimenDetail data
        if detail_msg:
            detail = detail_msg[0]
            record["status"] = "api_ephys_found"
            record["structure_area"] = detail.get("structure_parent__acronym") or detail.get("structure__acronym") or "n/a"
            record["structure_layer"] = detail.get("structure__layer") or "n/a"
            record["transgenic_line"] = detail.get("line_name") or "n/a"
            record["full_genotype"] = detail.get("donor__name") or "n/a"
            record["dendrite_type"] = detail.get("tag__dendrite_type") or "n/a"
            
            # Heuristic for cell class
            dendrite = record["dendrite_type"]
            line = record["transgenic_line"]
            if dendrite == "spiny":
                record["reported_cell_class"] = "excitatory"
            elif dendrite == "aspiny":
                record["reported_cell_class"] = "inhibitory"
            elif any(x in line for x in ["Pvalb", "Sst", "Vip"]):
                record["reported_cell_class"] = "inhibitory"
            elif any(x in line for x in ["Cux2", "Rorb", "Scnn1a", "Rbp4", "Nr5a1"]):
                record["reported_cell_class"] = "excitatory"
            else:
                record["reported_cell_class"] = "unknown"
                
            # Populate metrics from detail fields
            record["resting_membrane_potential_mv"] = detail.get("ef__vrest")
            record["input_resistance_mohm"] = detail.get("ef__ri")
            record["tau_ms"] = detail.get("ef__tau")
            record["rheobase_pa"] = detail.get("ef__threshold_i_long_square")
            record["firing_rate_hz"] = detail.get("ef__avg_firing_rate")
            record["adaptation_index"] = detail.get("ef__adaptation")
            record["upstroke_downstroke_ratio"] = detail.get("ef__upstroke_downstroke_ratio_long_square")
            
            # Model counts
            record["glif_model_count"] = detail.get("m__glif") or 0
            record["biophysical_model_count"] = (detail.get("m__biophys_perisomatic") or 0) + (detail.get("m__biophys_all_active") or 0)
            
            # NWB file
            if detail.get("erwkf__id"):
                record["nwb_availability"] = True
                
        # Process EphysFeature data for supplementary metrics
        if ephys_msg:
            ephys = ephys_msg[0]
            # Get threshold voltage
            record["threshold_voltage_mv"] = ephys.get("threshold_v_long_square")
            # If basic fields were missing in detail, fill them from ephys
            if record["resting_membrane_potential_mv"] is None or record["resting_membrane_potential_mv"] == "n/a":
                record["resting_membrane_potential_mv"] = ephys.get("vrest")
            if record["input_resistance_mohm"] is None or record["input_resistance_mohm"] == "n/a":
                record["input_resistance_mohm"] = ephys.get("ri") or ephys.get("input_resistance_mohm")
            if record["tau_ms"] is None or record["tau_ms"] == "n/a":
                record["tau_ms"] = ephys.get("tau")
            if record["rheobase_pa"] is None or record["rheobase_pa"] == "n/a":
                record["rheobase_pa"] = ephys.get("threshold_i_long_square")
            if record["adaptation_index"] is None or record["adaptation_index"] == "n/a":
                record["adaptation_index"] = ephys.get("adaptation")
            if record["upstroke_downstroke_ratio"] is None or record["upstroke_downstroke_ratio"] == "n/a":
                record["upstroke_downstroke_ratio"] = ephys.get("upstroke_downstroke_ratio_long_square")
                
            if record["status"] == "not_found_or_failed":
                record["status"] = "partial_api_data"
                record["notes"] += "Only EphysFeature database record found. "
                
        if record["status"] == "not_found_or_failed":
            record["notes"] += "ID not found in database or query failed. "
            record["spontaneous_activity_status"] = "unavailable"
        else:
            record["spontaneous_activity_status"] = "pending"
            record["notes"] += "Сырой NWB файл требует полной загрузки (100+ МБ), анализ спонтанной активности 0 pA отложен до запуска специализированного скрипта."

        records.append(record)

    # Ensure output directories exist
    os.makedirs("artifacts", exist_ok=True)
    os.makedirs("docs/engine/research", exist_ok=True)

    # 1. Save JSON
    json_path = "artifacts/reference_neuron_harvest.json"
    with open(json_path, "w", encoding="utf-8") as f:
        json.dump(records, f, indent=2, ensure_ascii=False)
    print(f"Saved JSON: {json_path}")

    # 2. Save CSV
    csv_path = "artifacts/reference_neuron_harvest.csv"
    headers = [
        "specimen_id", "status", "structure_area", "structure_layer", 
        "transgenic_line", "full_genotype", "dendrite_type", "reported_cell_class",
        "resting_membrane_potential_mv", "input_resistance_mohm", "tau_ms", "rheobase_pa",
        "threshold_voltage_mv", "firing_rate_hz", "adaptation_index", "upstroke_downstroke_ratio",
        "ap_half_width", "nwb_availability", "glif_model_count", "biophysical_model_count",
        "spontaneous_activity_status", "notes"
    ]
    with open(csv_path, "w", encoding="utf-8", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=headers)
        writer.writeheader()
        for r in records:
            writer.writerow(r)
    print(f"Saved CSV: {csv_path}")

    # 3. Generate Summary Markdown
    summary_path = "docs/engine/research/reference_neuron_harvest_summary.md"
    generate_summary_md(records, summary_path)
    print(f"Saved Summary MD: {summary_path}")
    print("Harvest completed successfully!")

def generate_summary_md(records, output_path):
    # Segregate records
    # Primary Calibration Pack must contain exactly: 313861608, 314900022, 471141261, 490376252, 324493977
    primary_ids = [313861608, 314900022, 471141261, 490376252, 324493977]
    
    calibration_pack = []
    contested_pack = []
    exclude_pack = []
    
    for r in records:
        sid = r["specimen_id"]
        status = r["status"]
        
        # Excluded cases (failed to load or specific classification conflict edge cases)
        if sid in [469801138, "L23_spiny_VISp23_1", "L23_aspiny_VISp23_1"] or status == "not_found_or_failed":
            exclude_pack.append(r)
            continue
            
        # Target Primary Pack
        if sid in primary_ids:
            if status == "api_ephys_found":
                calibration_pack.append(r)
            else:
                contested_pack.append(r)
        else:
            # Everything else goes to contested/backup review
            contested_pack.append(r)

    with open(output_path, "w", encoding="utf-8") as f:
        f.write("# Результаты сбора данных эталонных нейронов (Allen Harvest Summary)\n")
        f.write("*(reference-neuron-allen-harvest-v1)*\n\n")
        
        f.write("Данный документ суммирует результаты автоматического сбора электрофизиологических метрик и морфологических метаданных по 17 specimen ID кандидатов из Allen Cell Types Database REST API. Поля статуса и подтверждения отражают исключительно успешность скачивания данных из REST-интерфейса и не являются биологической верификацией.\n\n")
        
        f.write("## 1. Сводная таблица сбора данных\n\n")
        f.write("| Specimen ID | Статус сбора | Область/Слой | Линия (Cre) | Дендриты | Класс нейрона | $V_{\\text{rest}}$ (mV) | $R_{\\text{in}}$ (M$\\Omega$) | $\\tau_m$ (ms) | Реобаза (pA) | NWB | Модели (GLIF/Biophys) | Спонтанность |\n")
        f.write("|:---|:---|:---|:---|:---|:---|:---|:---|:---|:---|:---|:---|:---|\n")
        
        for r in records:
            v_rest = f"{r['resting_membrane_potential_mv']:.2f}" if isinstance(r['resting_membrane_potential_mv'], float) else str(r['resting_membrane_potential_mv'])
            ri = f"{r['input_resistance_mohm']:.2f}" if isinstance(r['input_resistance_mohm'], float) else str(r['input_resistance_mohm'])
            tau = f"{r['tau_ms']:.2f}" if isinstance(r['tau_ms'], float) else str(r['tau_ms'])
            rheo = f"{r['rheobase_pa']:.0f}" if isinstance(r['rheobase_pa'], float) else str(r['rheobase_pa'])
            nwb = "Есть" if r["nwb_availability"] else "Нет"
            models = f"{r['glif_model_count']} GLIF / {r['biophysical_model_count']} BP"
            
            f.write(f"| **{r['specimen_id']}** | {r['status']} | {r['structure_area']} L{r['structure_layer']} | {r['transgenic_line']} | {r['dendrite_type']} | {r['reported_cell_class']} | {v_rest} | {ri} | {tau} | {rheo} | {nwb} | {models} | {r['spontaneous_activity_status']} |\n")
            
        f.write("\n*Единицы измерения сохранены в исходном формате Allen Database (mV, MΩ, ms, pA, Hz). Поле статуса принимает значения: `api_ephys_found` (данные ephys и метаданные найдены), `partial_api_data` (данные найдены частично), `not_found_or_failed` (ID не найден или ошибка).* \n\n")
        
        f.write("## 2. Группы по калибровочной готовности\n\n")
        
        f.write("### 🟢 Калибровочный набор первого приоритета (Primary Calibration Pack)\n")
        f.write("Данные нейроны выбраны как первичные калибровочные референсы (seeds), для которых в базе REST API найдены ephys-поля и метаданные:\n\n")
        for r in calibration_pack:
            f.write(f"- **{r['specimen_id']}** ({r['reported_cell_class']} L{r['structure_layer']} {r['transgenic_line']}): $V_m$={r['resting_membrane_potential_mv']:.1f} mV, $R_i$={r['input_resistance_mohm']:.1f} M$\\Omega$, $\\tau$={r['tau_ms']:.1f} ms, Rheo={r['rheobase_pa']:.0f} pA.\n")
            
        f.write("\n### 🟡 Спорные, резервные и вспомогательные нейроны (Require review)\n")
        f.write("Данные кандидаты требуют дополнительного анализа, так как находятся вне первичной зрительной коры (VISp), имеют спорные роли или не относятся к первичному набору:\n\n")
        for r in contested_pack:
            reason = ""
            if r["specimen_id"] in [313862134, 486754703]:
                reason = "Недоказанная пейсмейкерная роль (требуется программный парсинг 0 pA sweeps)."
            elif r["specimen_id"] == 475549334:
                reason = "Модельный артефакт биофизического пакета (активная апикальная модель на aspiny геометрии)."
            elif r["structure_area"] not in ["VISp", "VISp2/3", "VISp4", "VISp5", "VISp6a"] and r["status"] != "not_found_or_failed":
                reason = f"Локализация вне VISp ({r['structure_area']}); возможны кросс-региональные сдвиги ephys-свойств."
            else:
                reason = "Переведен в резерв в соответствии с обновленной политикой доказательности."
            f.write(f"- **{r['specimen_id']}**: {reason}\n")
            
        f.write("\n### 🔴 Исключенные / Проблемные ID (Не использовать как baseline)\n")
        for r in exclude_pack:
            reason = ""
            if r["status"] == "not_found_or_failed":
                reason = "ID отсутствует в активной базе данных Allen Cell Types REST API."
            elif r["specimen_id"] == 469801138:
                reason = "classification conflict из предыдущего аудита требует отдельной проверки (ephys-класс возбуждающий не доказан текущим сбором, требуется анализ NWB)."
            else:
                reason = r["notes"]
            f.write(f"- **{r['specimen_id']}**: {reason}\n")
            
        f.write("\n## 3. Сверка утверждений из audit.md с фактическими данными API\n\n")
        
        # Check claims
        c1 = next((r for r in records if r["specimen_id"] == 313861608), None)
        f.write("1. **Метрики 313861608 (PV L5 FS)**: ")
        if c1 and c1["status"] == "api_ephys_found":
            f.write("✅ **REST API подтвердил наличие базовых метаданных/ephys-полей**. ")
            f.write(f"rest={c1['resting_membrane_potential_mv']:.2f} mV (в audit: -74.6), ri={c1['input_resistance_mohm']:.2f} M$\\Omega$ (в audit: 81.0), tau={c1['tau_ms']:.2f} ms (в audit: 22.6), rheo={c1['rheobase_pa']:.0f} pA (в audit: 290).\n")
        else:
            f.write("❌ **Ошибка запроса / Данные не найдены**.\n")
            
        c2 = next((r for r in records if r["specimen_id"] == 313862134), None)
        f.write("2. **Метрики 313862134 (Sst L5)**: ")
        if c2 and c2["status"] == "api_ephys_found":
            f.write("✅ **REST API подтвердил наличие базовых метаданных/ephys-полей**. ")
            f.write(f"rest={c2['resting_membrane_potential_mv']:.2f} mV (в audit: -74.11), ri={c2['input_resistance_mohm']:.2f} M$\\Omega$ (в audit: 219.22), tau={c2['tau_ms']:.2f} ms (в audit: 11.3), rheo={c2['rheobase_pa']:.0f} pA (в audit: 110).\n")
        else:
            f.write("❌ **Ошибка запроса / Данные не найдены**.\n")

        c3 = next((r for r in records if r["specimen_id"] == 517974394), None)
        f.write("3. **Кандидат L2/3 517974394 (Nr5a1)**: ")
        if c3 and c3["status"] == "api_ephys_found":
            f.write("✅ **REST API подтвердил наличие базовых метаданных/ephys-полей**. ")
            f.write(f"Клетка находится в VISp2/3, rest={c3['resting_membrane_potential_mv']:.2f} mV, ri={c3['input_resistance_mohm']:.2f} M$\\Omega$, tau={c3['tau_ms']:.2f} ms.\n")
        else:
            f.write("❌ **Ошибка запроса / Данные не найдены**.\n")

        c4 = next((r for r in records if r["specimen_id"] == 490944352), None)
        f.write("4. **Кандидат L5 490944352 (Rbp4)**: ")
        if c4 and c4["status"] != "not_found_or_failed":
            f.write("✅ **REST API подтвердил наличие базовых метаданных/ephys-полей**.\n")
        else:
            f.write("❌ **Опровергнуто / Отсутствует в БД**. ID не существует в активной базе данных Allen Cell Types REST API.\n")

        c5 = next((r for r in records if r["specimen_id"] == 313861411), None)
        f.write("5. **Существование 313861411 (PV L4)**: ")
        if c5 and c5["status"] == "api_ephys_found":
            f.write("✅ **REST API подтвердил наличие базовых метаданных/ephys-полей**. ")
            f.write(f"ID существует в базе, область: {c5['structure_area']}, Cre-линия: {c5['transgenic_line']}, дендриты: {c5['dendrite_type']}.\n")
        else:
            f.write("❌ **ID не найден**.\n")

        c6 = next((r for r in records if r["specimen_id"] == 469801138), None)
        f.write("6. **Конфликт классификации 469801138 (PV L4)**: ")
        f.write("⚠️ **Требуется отдельная проверка**. classification conflict из предыдущего аудита требует отдельного анализа (ephys-класс возбуждающий не доказан текущим сбором, требуется анализ NWB).\n")

        f.write("\n## 4. Что еще НЕ проверено\n\n")
        f.write("Ниже приведен список параметров и свойств, которые не могут быть подтверждены фактом успешного выполнения REST-запроса к базе метаданных и требуют отдельного анализа экспериментальных записей:\n\n")
        f.write("- **Спонтанная активность / 0 pA sweeps**: Действительное наличие или отсутствие спонтанного ритмического firing in vitro требует скачивания сырых NWB-файлов (100+ МБ) для каждого интересующего нейрона и парсинга разверток без инжекции тока.\n")
        f.write("- **Полуширина потенциала действия (AP half-width)**: Данный параметр отсутствует в плоской таблице вычисленных признаков EphysFeature и должен рассчитываться вручную по кривой потенциала первого спайка при реобазе.\n")
        f.write("- **Форма волны потенциала действия (AP waveform morphology)**: Профили деполяризации, реполяризации и послеспайковой гиперполяризации (AHP) не сверялись с NWB-развертками.\n")
        f.write("- **Реальная доступность и целостность NWB-свипов**: Фактическая скачиваемость файлов с серверов Allen и целостность NWB-структур не тестировались.\n")
        f.write("- **Содержимое конфигураций GLIF**: В рамках сбора подтверждено лишь количество зарегистрированных GLIF-моделей в БД, однако сами коэффициенты проводимости, правила сброса мембранного потенциала и токи адаптации требуют парсинга конфигурационных файлов моделей.\n")

        f.write("\n## 5. Следующие шаги калибровки\n\n")
        f.write("1. Использовать CSV-датасет `artifacts/reference_neuron_harvest.csv` как основу для тестового окружения калибровки.\n")
        f.write("2. Для спорных нейронов разработать скрипт скачивания сырых NWB-файлов и автоматического парсинга разверток при инжекции 0 пА, чтобы вынести вердикт по спонтанной активности.\n")
        f.write("3. Запустить скриптовую проверку параметров GLIF-моделей для набора первого приоритета (Primary Calibration Pack).\n")

if __name__ == "__main__":
    main()
