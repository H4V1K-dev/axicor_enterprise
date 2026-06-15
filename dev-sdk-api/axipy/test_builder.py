import unittest
import tempfile
import shutil
from pathlib import Path
import toml

from axipy.builder import ModelBuilder, Shard, Department, NeuronBlueprint, Socket
from axipy.errors import (
    InvalidDensityError,
    InvalidEntryZError,
    InvalidAnatomyHeightError,
    InvalidLayerCompositionError,
    NeuronTypeLimitExceededError,
    PhysicsDriftError,
    SocketConnectionError,
    BuilderValidationError,
)

class TestAxipyBuilder(unittest.TestCase):
    
    def setUp(self):
        # Create a temporary directory for build output
        self.test_dir = tempfile.mkdtemp()
        self.output_dir = Path(self.test_dir) / "output"
        
        # Create a dummy neuron blueprint TOML file
        self.blueprint_path = Path(self.test_dir) / "dummy_neuron.toml"
        self.blueprint_data = {
            "neuron": {
                "name": "DummyNeuron",
                "threshold": 1000
            }
        }
        with open(self.blueprint_path, "w", encoding="utf-8") as f:
            toml.dump(self.blueprint_data, f)
            
    def tearDown(self):
        shutil.rmtree(self.test_dir)

    def test_negative_density(self):
        shard = Shard("TestShard", 10, 10, 10)
        with self.assertRaises(InvalidDensityError):
            shard.add_layer("Layer1", 1.0, -0.5)

    def test_invalid_entry_z(self):
        shard = Shard("TestShard", 10, 10, 10)
        
        # Valid values
        shard.add_input_port("Port1", 4, 4, "top")
        shard.add_input_port("Port2", 4, 4, 0.5)
        shard.add_socket("Socket1", 4, 4, "bottom")
        
        # Invalid string
        with self.assertRaises(InvalidEntryZError):
            shard.add_input_port("Port3", 4, 4, "invalid_label")
            
        # Invalid numeric values
        with self.assertRaises(InvalidEntryZError):
            shard.add_socket("Socket2", 4, 4, 1.2)
        with self.assertRaises(InvalidEntryZError):
            shard.add_socket("Socket3", 4, 4, -0.1)

    def test_layer_heights_fraction(self):
        builder = ModelBuilder("Proj", str(self.output_dir))
        bp = builder.gnm_lib(str(self.blueprint_path))
        
        shard = Shard("Shard1", 10, 10, 10)
        shard.add_layer("L1", 0.4, 1.0)
        shard.add_layer("L2", 0.5, 1.0) # Sum is 0.9 (not 1.0)
        shard.add_population("L1", bp, 1.0)
        shard.add_population("L2", bp, 1.0)
        
        dept = Department("Dept1").add_shard(shard)
        builder.add_department(dept)
        
        with self.assertRaises(InvalidAnatomyHeightError):
            builder.build(dry_run=True)

    def test_population_fractions_sum(self):
        builder = ModelBuilder("Proj", str(self.output_dir))
        bp = builder.gnm_lib(str(self.blueprint_path))
        
        shard = Shard("Shard1", 10, 10, 10)
        shard.add_layer("L1", 1.0, 1.0)
        shard.add_population("L1", bp, 0.8) # Sum is 0.8 (not 1.0)
        
        dept = Department("Dept1").add_shard(shard)
        builder.add_department(dept)
        
        with self.assertRaises(InvalidLayerCompositionError):
            builder.build(dry_run=True)

    def test_shard_neuron_types_limit(self):
        builder = ModelBuilder("Proj", str(self.output_dir))
        
        # Create 17 unique blueprints to trigger limit of 16
        blueprints = []
        for i in range(17):
            bp_path = Path(self.test_dir) / f"bp_{i}.toml"
            with open(bp_path, "w", encoding="utf-8") as f:
                toml.dump({"name": f"bp_{i}"}, f)
            blueprints.append(builder.gnm_lib(str(bp_path)))
            
        shard = Shard("Shard1", 10, 10, 10)
        shard.add_layer("L1", 1.0, 1.0)
        # Add 17 populations to the same layer
        # Sum of fractions must be 1.0
        frac = 1.0 / 17.0
        for bp in blueprints:
            shard.add_population("L1", bp, frac)
            
        # Adjust sum fraction to precisely 1.0 for layers composition check
        shard.layers[0].populations[-1] = (blueprints[-1], 1.0 - frac * 16)
        
        dept = Department("Dept1").add_shard(shard)
        builder.add_department(dept)
        
        with self.assertRaises(NeuronTypeLimitExceededError):
            builder.build(dry_run=True)

    def test_physics_drift_validation(self):
        builder = ModelBuilder("Proj", str(self.output_dir))
        bp = builder.gnm_lib(str(self.blueprint_path))
        
        shard = Shard("Shard1", 10, 10, 10)
        shard.add_layer("L1", 1.0, 1.0)
        shard.add_population("L1", bp, 1.0)
        dept = Department("Dept1").add_shard(shard)
        builder.add_department(dept)
        
        # Set parameters that trigger PhysicsDriftError
        # v_seg_raw = (signal_speed * tick_duration) / (voxel_size * segment_length)
        # We want v_seg_raw to be non-integer, e.g. 1.25
        builder.sim_params = {
            "signal_speed_m_s": 2.5,
            "tick_duration_us": 1000.0,
            "voxel_size_um": 1000.0,
            "segment_length_voxels": 2
        }
        # v_seg_raw = 2.5 * 1000 / (1000 * 2) = 2500 / 2000 = 1.25 -> PhysicsDriftError
        
        with self.assertRaises(PhysicsDriftError) as ctx:
            builder.build(dry_run=True)
            
        self.assertAlmostEqual(ctx.exception.raw_v_seg, 1.25)
        # Round 1.25 is 1.0. Suggested speed: (1.0 * 1000 * 2) / 1000 = 2.0
        self.assertAlmostEqual(ctx.exception.suggested_speed, 2.0)

    def test_socket_dimension_mismatch(self):
        builder = ModelBuilder("Proj", str(self.output_dir))
        bp = builder.gnm_lib(str(self.blueprint_path))
        
        shard1 = Shard("Shard1", 10, 10, 10)
        shard1.add_layer("L1", 1.0, 1.0)
        shard1.add_population("L1", bp, 1.0)
        # Outgoing socket: width 4, height 4
        s1 = shard1.add_socket("SocketOut", 4, 4, entry_z=None)
        
        shard2 = Shard("Shard2", 10, 10, 10)
        shard2.add_layer("L1", 1.0, 1.0)
        shard2.add_population("L1", bp, 1.0)
        # Incoming socket: width 5, height 4 (mismatch)
        shard2.add_socket("SocketIn", 5, 4, entry_z="mid")
        
        s1.connect_to("Shard2.SocketIn")
        
        dept = Department("Dept1").add_shard(shard1, shard2)
        builder.add_department(dept)
        
        with self.assertRaises(BuilderValidationError):
            builder.build(dry_run=True)

    def test_socket_connection_errors(self):
        builder = ModelBuilder("Proj", str(self.output_dir))
        bp = builder.gnm_lib(str(self.blueprint_path))
        
        shard1 = Shard("Shard1", 10, 10, 10)
        shard1.add_layer("L1", 1.0, 1.0)
        shard1.add_population("L1", bp, 1.0)
        s1 = shard1.add_socket("SocketOut", 4, 4, entry_z=None)
        
        shard2 = Shard("Shard2", 10, 10, 10)
        shard2.add_layer("L1", 1.0, 1.0)
        shard2.add_population("L1", bp, 1.0)
        # Incoming socket without connection (orphan)
        shard2.add_socket("SocketIn", 4, 4, entry_z="mid")
        
        dept = Department("Dept1").add_shard(shard1, shard2)
        builder.add_department(dept)
        
        # Test orphan error
        with self.assertRaises(SocketConnectionError) as ctx:
            builder.build(dry_run=True)
        self.assertIn("orphan", str(ctx.exception))
        
        # Test double connection error
        # Connect two outgoing sockets to the same incoming socket
        s1.connect_to("Shard2.SocketIn")
        s2 = shard1.add_socket("SocketOut2", 4, 4, entry_z=None)
        s2.connect_to("Shard2.SocketIn")
        
        with self.assertRaises(SocketConnectionError) as ctx:
            builder.build(dry_run=True)
        self.assertIn("multiple", str(ctx.exception))

    def test_successful_build_and_stats(self):
        builder = ModelBuilder("Proj", str(self.output_dir))
        bp = builder.gnm_lib(str(self.blueprint_path))
        
        # Setup Shard 1 (day phase interface and outputs)
        shard1 = Shard("Shard1", 10, 10, 10)
        shard1.add_layer("L1", 0.3, 2.0)
        shard1.add_layer("L2", 0.7, 1.5)
        shard1.add_population("L1", bp, 1.0)
        shard1.add_population("L2", bp, 1.0)
        shard1.add_input_port("Input1", 8, 8, "top")
        s1 = shard1.add_socket("SocketOut", 4, 4, entry_z=None)
        
        # Setup Shard 2 (night phase connections)
        shard2 = Shard("Shard2", 5, 5, 20)
        shard2.add_layer("L1", 1.0, 1.0)
        shard2.add_population("L1", bp, 1.0)
        shard2.add_socket("SocketIn", 4, 4, entry_z="mid")
        
        s1.connect_to("Shard2.SocketIn")
        
        dept = Department("Dept1").add_shard(shard1, shard2)
        builder.add_department(dept)
        
        # Build should succeed
        builder.build(dry_run=False)
        
        # Verify TOML files exist and are correct
        self.assertTrue((self.output_dir / "simulation.toml").exists())
        self.assertTrue((self.output_dir / "Dept1.toml").exists())
        self.assertTrue((self.output_dir / "model_pass.md").exists())
        
        # Check resource estimates
        stats = builder.dry_run_stats()
        
        # Shard1 estimate neurons:
        # L1: 10 * 10 * (10 * 0.3) * 2.0 = 100 * 3 * 2.0 = 600
        # L2: 10 * 10 * (10 * 0.7) * 1.5 = 100 * 7 * 1.5 = 1050
        # Total Shard1 estimate = 1650
        
        # Shard2 estimate neurons:
        # L1: 5 * 5 * 20 * 1.0 = 500
        
        # Total Neurons = 1650 + 500 = 2150
        self.assertEqual(stats["total_neurons"], 2150)
        
        # Test correct memory estimation
        from axipy.contract import NEURON_SIZE, AXON_SIZE
        self.assertEqual(stats["vram_neurons_bytes"], 2150 * NEURON_SIZE)
        self.assertTrue(stats["vram_axons_bytes"] > 0)
        self.assertTrue(stats["ipc_buffer_overhead_bytes"] > 0)

if __name__ == "__main__":
    unittest.main()
