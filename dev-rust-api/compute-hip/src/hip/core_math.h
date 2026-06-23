#pragma once
#include <stdint.h>

// INV-COMPUTE-HIP-003: Math Consistency
// INV-PHYS-005: Zero-Float. All math must use fixed-point integer arithmetic.

#define GSOP_FIXED_POINT_SHIFT 7
#define INERTIA_RANK_SHIFT 28
#define MAX_WEIGHT_LIMIT 2140000000

// Branchless restrict to max(0, val)
__device__ __forceinline__ int32_t compute_glif(int32_t voltage, int32_t rest_potential, uint32_t leak_shift, int32_t input_current) {
    uint32_t v_int_u = (uint32_t)voltage + (uint32_t)input_current;
    uint32_t diff = v_int_u - (uint32_t)rest_potential;
    int32_t diff_signed = (int32_t)diff;
    return (int32_t)(v_int_u - (uint32_t)(diff_signed >> leak_shift));
}

__device__ __forceinline__ int32_t update_homeostasis(int32_t offset, uint16_t decay, bool is_spiking, int32_t penalty) {
    uint32_t decayed_u = (uint32_t)offset - (uint32_t)decay;
    int32_t decayed = (int32_t)decayed_u;
    int32_t clamped = decayed & ~(decayed >> 31);
    uint32_t penalty_u = is_spiking ? (uint32_t)penalty : 0;
    return (int32_t)((uint32_t)clamped + penalty_u);
}

__device__ __forceinline__ uint32_t inertia_rank(int32_t abs_weight) {
    uint32_t shifted = ((uint32_t)abs_weight) >> INERTIA_RANK_SHIFT;
    return shifted < 7 ? shifted : 7;
}

__device__ __forceinline__ int32_t compute_gsop_weight(
    int32_t weight,
    int16_t dopamine,
    uint8_t d1_aff,
    uint8_t d2_aff,
    uint16_t pot,
    uint16_t dep,
    int32_t inertia,
    bool is_active,
    int32_t burst_mult,
    uint32_t cooling_shift
) {
    int32_t sign = weight >= 0 ? 1 : -1;
    uint32_t abs_w = weight >= 0 ? (uint32_t)weight : ~(uint32_t)weight + 1;

    int32_t pot_mod = ((int32_t)dopamine * (int32_t)d1_aff) >> GSOP_FIXED_POINT_SHIFT;
    int32_t dep_mod = ((int32_t)dopamine * (int32_t)d2_aff) >> GSOP_FIXED_POINT_SHIFT;

    int32_t raw_pot = (int32_t)pot + pot_mod;
    int32_t raw_dep = (int32_t)dep - dep_mod;
    int32_t final_pot = raw_pot & ~(raw_pot >> 31);
    int32_t final_dep = raw_dep & ~(raw_dep >> 31);

    int32_t delta_pot = (final_pot * inertia * burst_mult) >> GSOP_FIXED_POINT_SHIFT;
    int32_t delta_dep = (final_dep * inertia * burst_mult) >> GSOP_FIXED_POINT_SHIFT;

    int32_t delta = 0;
    if (is_active) {
        uint32_t shift = cooling_shift < 31 ? cooling_shift : 31;
        delta = delta_pot >> shift;
    } else {
        delta = -delta_dep;
    }
    delta = (delta * 128) >> GSOP_FIXED_POINT_SHIFT;

    uint32_t new_abs_u = abs_w + (uint32_t)delta;
    int32_t new_abs = (int32_t)new_abs_u;
    new_abs &= ~(new_abs >> 31);
    new_abs = new_abs < MAX_WEIGHT_LIMIT ? new_abs : MAX_WEIGHT_LIMIT;

    return new_abs * sign;
}
