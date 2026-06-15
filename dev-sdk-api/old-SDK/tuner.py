from collections import deque
from enum import Enum
from .control import AxicorControl

class Phase(Enum):
    EXPLORATION = 1   # Rapid growth, high dopamine, low pruning threshold
    DISTILLATION = 2  # Noise burnout, high pruning threshold, frequent sleep
    CRYSTALLIZED = 3  # Pure inference, sleep and plasticity disabled

class AxicorAutoTuner:
    """
    Production State Machine for automated topology distillation.
    Operates on Simple Moving Average (SMA) of environment metrics.
    """
    def __init__(self, control: AxicorControl, target_score: float = 400.0, window_size: int = 15, **kwargs):
        self.control = control
        # [DOD FIX] Explicit initialization for better type inference
        self.default_target = target_score
        self.window = deque(maxlen=window_size)
        self.phase = Phase.EXPLORATION
        
        self.target_score = 400.0

        # Phase-specific parameter configuration
        self.explore_params = self._extract_params(kwargs, "explore_")
        self.distill_params = self._extract_params(kwargs, "distill_")
        self.crystallized_params = self._extract_params(kwargs, "crystallized_")

        # Initialize exploration phase
        self._apply_phase_settings(self.explore_params)

    def _extract_params(self, kwargs: dict, prefix: str) -> dict:
        """Extracts ALL parameters with the given prefix. Defaults to None."""
        params = {}
        for key, value in kwargs.items():
            if key.startswith(prefix):
                param_name = key[len(prefix):]
                params[param_name] = value
        # target_score -> target (for backward compatibility)
        if "target_score" in params:
            params["target"] = params.pop("target_score")
        if "target" not in params:
            params["target"] = self.default_target
        return params

    def _apply_phase_settings(self, p: dict):
        """Propagates parameters to the manifest. Any parameter can be None  simply skipped."""
        if p.get("prune") is not None:
            self.control.set_prune_threshold(p["prune"])
        if p.get("night") is not None:
            self.control.set_night_interval(p["night"])
        if p.get("sprouts") is not None:
            self.control.set_max_sprouts(p["sprouts"])
        
        # Receptors (variant 0/1)
        if p.get("d1") is not None or p.get("d2") is not None:
            self.control.set_dopamine_receptors(0, p.get("d1"), p.get("d2"))
            self.control.set_dopamine_receptors(1, p.get("d1"), p.get("d2"))
        
        # Physics
        if p.get("leak") is not None or p.get("homeos_penalty") is not None or p.get("homeos_decay") is not None:
            l_base = p.get("leak")
            hp_base = p.get("homeos_penalty")
            hd_base = p.get("homeos_decay")

            l_inh = int(l_base * 1.5) if l_base is not None else None
            hp_inh = int(hp_base * 0.8) if hp_base is not None else None

            self.control.set_membrane_physics(0, l_base, hp_base, hd_base)
            self.control.set_membrane_physics(1, l_inh, hp_inh, hd_base)

        # [DOD FIX] Flat caching of variables for O(1) access in Hot Loop
        if p.get("target") is not None: self.target_score = p["target"]


    def step(self, episode_score: float) -> Phase:
        """
        Called at the end of each environment episode.
        Returns the current phase for logging.
        """
        if self.phase == Phase.CRYSTALLIZED:
            return self.phase
            
        self.window.append(episode_score)
        if len(self.window) < self.window.maxlen:
            return self.phase # Wait for enough statistics
            
        sma_score = sum(self.window) / len(self.window)
        
        if self.phase == Phase.EXPLORATION:
            # Network found a potential solution (e.g., stable 70% of target)
            if sma_score >= self.target_score * 0.7:
                self._transition_to_distillation()
                
        elif self.phase == Phase.DISTILLATION:
            # Network cleared noise and reached the ideal state
            if sma_score >= self.target_score:
                self._transition_to_crystallization()
            # Network lost the skill due to excessive pruning
            elif sma_score < self.target_score * 0.4:
                self._transition_to_exploration()
                
        return self.phase

    def _transition_to_distillation(self):
        print("\n [AutoTuner] Transitioning to DISTILLATION: Burning weak connections and refining physics...")
        self._apply_phase_settings(self.distill_params)
        self.phase = Phase.DISTILLATION

    def _transition_to_crystallization(self):
        print("\n [AutoTuner] Transitioning to CRYSTALLIZED: Graph frozen. Ideal skill level.")
        self._apply_phase_settings(self.crystallized_params)
        
        # [DOD FIX] Hardware-level disabling of synaptic plasticity (GSOP)
        self.control.disable_all_plasticity() 
        self.phase = Phase.CRYSTALLIZED

    def _transition_to_exploration(self):
        print("\n [AutoTuner] Rolling back to EXPLORATION: Skill lost, resuming growth...")
        self._apply_phase_settings(self.explore_params)
        self.phase = Phase.EXPLORATION
        self.window.clear() # Reset window to prevent oscillation
