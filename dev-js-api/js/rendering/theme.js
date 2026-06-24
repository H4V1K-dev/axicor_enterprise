/**
 * @fileoverview theme.js — Global visual theme configuration for CAD editor.
 * Houses opacity, color, and size parameters for consistent styling.
 */

export const THEME = {
  levelWireframe: {
    activeOpacity: 0.85,
    inactiveOpacity: 0.02,
    defaultOpacity: 0.18
  },
  deptWireframe: {
    activeOpacity: 0.7,
    inactiveOpacity: 0.03,
    defaultOpacity: 0.25
  },
  shard: {
    // Opacity values for different focus states
    activeLevelOpacity: 1.0,
    inactiveLevelOpacity: 0.05,
    selectedDimmedOpacity: 0.08,
    selectedConnectedOpacity: 0.5,
    modeDimmedOpacity: 0.15
  },
  label: {
    activeLevelOpacity: 0.65,
    inactiveLevelOpacity: 0.0
  },
  socket: {
    activeLevelOpacity: 1.0,
    inactiveLevelOpacity: 0.02,
    dimmedOpacity: 0.12,
    highlightBackingOpacity: 0.75,
    dimmedBackingOpacity: 0.1,
    defaultBackingOpacity: 0.7
  }
};
