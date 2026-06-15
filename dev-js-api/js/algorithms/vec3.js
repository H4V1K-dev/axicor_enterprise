/**
 * @fileoverview Pure math utilities for 3D vectors represented as plain objects {x, y, z}.
 * Zero dependencies, highly portable.
 */

/** @typedef {import("../contracts/types.js").Vec3} Vec3 */

/**
 * Adds two vectors.
 * @param {Vec3} a
 * @param {Vec3} b
 * @returns {Vec3}
 */
export function vec3_add(a, b) {
  return { x: a.x + b.x, y: a.y + b.y, z: a.z + b.z };
}

/**
 * Subtracts vector b from vector a.
 * @param {Vec3} a
 * @param {Vec3} b
 * @returns {Vec3}
 */
export function vec3_sub(a, b) {
  return { x: a.x - b.x, y: a.y - b.y, z: a.z - b.z };
}

/**
 * Scales a vector by a scalar.
 * @param {Vec3} v
 * @param {number} s
 * @returns {Vec3}
 */
export function vec3_scale(v, s) {
  return { x: v.x * s, y: v.y * s, z: v.z * s };
}

/**
 * Computes the dot product of two vectors.
 * @param {Vec3} a
 * @param {Vec3} b
 * @returns {number}
 */
export function vec3_dot(a, b) {
  return a.x * b.x + a.y * b.y + a.z * b.z;
}

/**
 * Computes the length of a vector.
 * @param {Vec3} v
 * @returns {number}
 */
export function vec3_len(v) {
  return Math.sqrt(v.x * v.x + v.y * v.y + v.z * v.z);
}

/**
 * Computes the distance between two vectors.
 * @param {Vec3} a
 * @param {Vec3} b
 * @returns {number}
 */
export function vec3_dist(a, b) {
  const dx = a.x - b.x;
  const dy = a.y - b.y;
  const dz = a.z - b.z;
  return Math.sqrt(dx * dx + dy * dy + dz * dz);
}

/**
 * Normalizes a vector. Returns a new zero vector if input length is zero.
 * @param {Vec3} v
 * @returns {Vec3}
 */
export function vec3_normalize(v) {
  const len = vec3_len(v);
  if (len === 0) {
    return { x: 0, y: 0, z: 0 };
  }
  return { x: v.x / len, y: v.y / len, z: v.z / len };
}

/**
 * Linearly interpolates between two vectors.
 * @param {Vec3} a
 * @param {Vec3} b
 * @param {number} t
 * @returns {Vec3}
 */
export function vec3_lerp(a, b, t) {
  return {
    x: a.x + (b.x - a.x) * t,
    y: a.y + (b.y - a.y) * t,
    z: a.z + (b.z - a.z) * t
  };
}

/**
 * Clones a vector.
 * @param {Vec3} v
 * @returns {Vec3}
 */
export function vec3_clone(v) {
  return { x: v.x, y: v.y, z: v.z };
}
