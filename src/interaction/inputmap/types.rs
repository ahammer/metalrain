use bevy::prelude::*;
use smallvec::SmallVec;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActionKind { Binary, Axis1, Axis2, Gesture }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ActionId(pub u16); // internal index (array position)

#[derive(Debug, Clone)]
pub struct ActionMeta { pub id: ActionId, pub name: String, pub description: String, pub kind: ActionKind }

#[derive(Default, Debug, Clone, Copy)]
pub struct ActionStateBinary { pub pressed: bool, pub just_pressed: bool, pub just_released: bool }
impl ActionStateBinary { pub fn clear_transitions(&mut self) { self.just_pressed = false; self.just_released = false; } }

#[derive(Default, Debug, Clone, Copy)]
pub struct ActionStateAxis1 { pub value: f32, pub delta: f32, pub active: bool, pub just_pressed: bool, pub just_released: bool }
impl ActionStateAxis1 { pub fn clear_transitions(&mut self) { self.just_pressed = false; self.just_released = false; self.delta = 0.0; } }

#[derive(Default, Debug, Clone, Copy)]
pub struct ActionStateAxis2 { pub value: Vec2, pub delta: Vec2, pub active: bool, pub just_pressed: bool, pub just_released: bool }
impl ActionStateAxis2 { pub fn clear_transitions(&mut self) { self.just_pressed = false; self.just_released = false; self.delta = Vec2::ZERO; } }

#[derive(Debug, Clone)]
pub enum ActionDynamicState { Binary(ActionStateBinary), Axis1(ActionStateAxis1), Axis2(ActionStateAxis2), Gesture(ActionStateBinary) }
impl ActionDynamicState {
    pub fn as_binary_mut(&mut self) -> Option<&mut ActionStateBinary> { match self { Self::Binary(b)|Self::Gesture(b)=>Some(b), _=>None } }
    pub fn as_axis1_mut(&mut self) -> Option<&mut ActionStateAxis1> { match self { Self::Axis1(a)=>Some(a), _=>None } }
    pub fn as_axis2_mut(&mut self) -> Option<&mut ActionStateAxis2> { match self { Self::Axis2(a)=>Some(a), _=>None } }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RawBindingToken { Key(KeyCode), MouseBtn(MouseButton), MouseMoveX, MouseMoveY, WheelVert, WheelHorz, TouchTap, TouchDrag }

#[derive(Debug, Clone)]
pub struct Binding { pub id: u32, pub tokens: SmallVec<[RawBindingToken; 2]>, pub hold_secs: f32 }

#[derive(Debug, Default, Clone)]
pub struct BindingRuntime { pub active: bool, pub just_pressed: bool, pub just_released: bool, pub hold_elapsed: f32 }

#[derive(Debug, Clone)]
pub struct VirtualAxis { pub name: String, pub pos: RawBindingToken, pub neg: RawBindingToken, pub scale: f32 }

#[derive(Debug, Clone, Default)]
pub struct GestureConfig { pub tap_max_time: f32, pub tap_max_move: f32, pub drag_min_move: f32, pub drag_smoothing: f32 }

#[derive(Debug, Default, Clone)]
pub struct GestureRuntime { pub pointer_down: bool, pub pointer_start: Vec2, pub pointer_last: Vec2, pub time_down: f32, pub max_moved: f32, pub dragging: bool, pub drag_axis_value: Vec2, pub drag_axis_raw_delta: Vec2 }

#[derive(Resource, Debug, Default)]
pub struct InputMap {
    pub actions: Vec<ActionMeta>,
    pub name_to_id: HashMap<String, ActionId>,
    pub bindings_index: HashMap<ActionId, Vec<u32>>, // mapping to binding ids
    pub bindings: Vec<Binding>,
    pub bindings_runtime: Vec<BindingRuntime>,
    pub chord_map: HashMap<SmallVec<[RawBindingToken; 2]>, u32>,
    pub dynamic_states: Vec<ActionDynamicState>,
    pub virtual_axes: Vec<VirtualAxis>,
    pub gesture_cfg: GestureConfig,
    pub gesture_rt: GestureRuntime,
    pub frame_counter: u64,
}

impl InputMap {
    pub fn get_state(&self, name: &str) -> Option<&ActionDynamicState> { self.name_to_id.get(name).map(|id| &self.dynamic_states[id.0 as usize]) }
    fn get_state_mut(&mut self, name: &str) -> Option<&mut ActionDynamicState> { let id = self.name_to_id.get(name)?; self.dynamic_states.get_mut(id.0 as usize) }
    pub fn pressed(&self, name: &str) -> bool { match self.get_state(name) { Some(ActionDynamicState::Binary(b)|ActionDynamicState::Gesture(b)) => b.pressed, Some(ActionDynamicState::Axis1(a)) => a.active, Some(ActionDynamicState::Axis2(a)) => a.active, None => false } }
    pub fn just_pressed(&self, name: &str) -> bool { match self.get_state(name) { Some(ActionDynamicState::Binary(b)|ActionDynamicState::Gesture(b)) => b.just_pressed, Some(ActionDynamicState::Axis1(a)) => a.just_pressed, Some(ActionDynamicState::Axis2(a)) => a.just_pressed, None => false } }
    pub fn just_released(&self, name: &str) -> bool { match self.get_state(name) { Some(ActionDynamicState::Binary(b)|ActionDynamicState::Gesture(b)) => b.just_released, Some(ActionDynamicState::Axis1(a)) => a.just_released, Some(ActionDynamicState::Axis2(a)) => a.just_released, None => false } }
    pub fn axis1(&self, name: &str) -> f32 { match self.get_state(name) { Some(ActionDynamicState::Axis1(a)) => a.value, _ => 0.0 } }
    pub fn axis2(&self, name: &str) -> Vec2 { match self.get_state(name) { Some(ActionDynamicState::Axis2(a)) => a.value, _ => Vec2::ZERO } }
    pub fn axis2_mut(&mut self, name: &str) -> Option<&mut ActionStateAxis2> { self.get_state_mut(name).and_then(|s| s.as_axis2_mut()) }
}
