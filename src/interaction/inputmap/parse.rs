use super::types::*;
use bevy::prelude::*;
use smallvec::SmallVec;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Default)]
pub struct ParsedInputConfig { pub input_map: InputMap, pub errors: Vec<String> }

#[derive(Debug, serde::Deserialize)]
struct ActionsSection(HashMap<String, ActionDecl>);
#[derive(Debug, serde::Deserialize)]
struct ActionDecl { description: Option<String>, kind: Option<String> }

#[derive(Debug, serde::Deserialize)]
struct BindingsSection(HashMap<String, Vec<String>>);

#[derive(Debug, serde::Deserialize)]
struct GestureToml { tap_max_time: Option<f32>, tap_max_move: Option<f32>, drag_min_move: Option<f32>, drag_smoothing: Option<f32> }

#[derive(Debug, serde::Deserialize)]
struct VirtualAxisToml { name: String, pos: String, neg: String, scale: Option<f32> }

#[derive(Debug, serde::Deserialize)]
struct RootToml { actions: Option<HashMap<String, ActionDecl>>, bindings: Option<HashMap<String, Vec<String>>>, #[serde(rename = "debug.bindings")] debug_bindings: Option<HashMap<String, Vec<String>>>, virtual_axes: Option<Vec<VirtualAxisToml>>, gesture: Option<GestureToml> }

pub fn parse_input_toml(raw: &str, debug_layer: bool) -> ParsedInputConfig {
    let mut result = ParsedInputConfig::default();
    let root: RootToml = match toml::from_str(raw) { Ok(r) => r, Err(e) => { result.errors.push(format!("Top-level parse: {e}")); return result; } };
    let mut actions: Vec<ActionMeta> = Vec::new();
    let mut name_to_id = HashMap::new();
    if let Some(map) = root.actions { for (idx, (name, decl)) in map.into_iter().enumerate() { if !validate_action_name(&name) { result.errors.push(format!("Invalid action name '{}': must be PascalCase", name)); continue; } let kind = decl.kind.as_deref().unwrap_or("Binary"); let kind_enum = match kind { "Binary" => ActionKind::Binary, "Axis1" => ActionKind::Axis1, "Axis2" => ActionKind::Axis2, "Gesture" => ActionKind::Gesture, other => { result.errors.push(format!("Action {} unknown kind '{}': expected Binary|Axis1|Axis2|Gesture", name, other)); ActionKind::Binary } }; let id = ActionId(idx as u16); actions.push(ActionMeta { id, name: name.clone(), description: decl.description.unwrap_or_default(), kind: kind_enum }); name_to_id.insert(name, id); } }

    // Allocate dynamic state storage
    let mut dynamic_states: Vec<ActionDynamicState> = Vec::with_capacity(actions.len());
    for meta in &actions { match meta.kind { ActionKind::Binary => dynamic_states.push(ActionDynamicState::Binary(ActionStateBinary::default())), ActionKind::Gesture => dynamic_states.push(ActionDynamicState::Gesture(ActionStateBinary::default())), ActionKind::Axis1 => dynamic_states.push(ActionDynamicState::Axis1(ActionStateAxis1::default())), ActionKind::Axis2 => dynamic_states.push(ActionDynamicState::Axis2(ActionStateAxis2::default())), } }

    let mut input_map = InputMap { actions: actions.clone(), name_to_id, dynamic_states, ..Default::default() };

    // Merge bindings (normal + debug overlay if allowed)
    let mut all_bindings: HashMap<String, Vec<String>> = HashMap::new();
    if let Some(b) = root.bindings { for (k,v) in b { all_bindings.entry(k).or_default().extend(v); } }
    if debug_layer { if let Some(db) = root.debug_bindings { for (k, v) in db { all_bindings.entry(k).or_default().extend(v); } } }

    let mut binding_id: u32 = 0;
    for (action_name, list) in all_bindings { let Some(aid) = input_map.name_to_id.get(&action_name).copied() else { result.errors.push(format!("Binding references unknown action '{}'" , action_name)); continue; }; for (_line_offset, spec) in list.iter().enumerate() { match parse_binding(spec) { Ok((mut tokens, hold)) => { canonical_sort_tokens(&mut tokens); let binding = Binding { id: binding_id, tokens: tokens.clone(), hold_secs: hold }; input_map.bindings_index.entry(aid).or_default().push(binding_id); input_map.chord_map.insert(tokens, binding_id); input_map.bindings.push(binding); binding_id += 1; }, Err(err) => { result.errors.push(format!("[binding {} '{}'] {err}", action_name, spec)); } } } }

    // Virtual axes
    if let Some(vas) = root.virtual_axes { for va in vas { match (parse_single_token(&va.pos), parse_single_token(&va.neg)) { (Ok(p), Ok(n)) => { input_map.virtual_axes.push(VirtualAxis { name: va.name, pos: p, neg: n, scale: va.scale.unwrap_or(1.0) }); }, (Err(e), _) => result.errors.push(format!("VirtualAxis pos error '{}': {e}", va.name)), (_, Err(e)) => result.errors.push(format!("VirtualAxis neg error '{}': {e}", va.name)), } } }

    // Gesture config
    if let Some(g) = root.gesture { input_map.gesture_cfg = GestureConfig { tap_max_time: g.tap_max_time.unwrap_or(0.25), tap_max_move: g.tap_max_move.unwrap_or(12.0), drag_min_move: g.drag_min_move.unwrap_or(4.0), drag_smoothing: g.drag_smoothing.unwrap_or(0.15) }; }
    result.input_map = input_map; result
}

fn validate_action_name(name: &str) -> bool { let bytes = name.as_bytes(); if bytes.is_empty() { return false; } if !bytes[0].is_ascii_uppercase() { return false; } !name.chars().any(|c| !c.is_ascii_alphanumeric()) }

fn canonical_sort_tokens(tokens: &mut SmallVec<[RawBindingToken;2]>) { tokens.sort_by_key(|t| token_sort_key(t)); }

fn token_sort_key(t: &RawBindingToken) -> (u8, u16) { use RawBindingToken::*; match t { Key(k) => (0, keycode_rank(*k)), MouseBtn(b) => (1, mouse_rank(*b)), MouseMoveX => (2, 0), MouseMoveY => (2, 1), WheelVert => (3, 0), WheelHorz => (3, 1), TouchTap => (4, 0), TouchDrag => (4, 1) } }

fn keycode_rank(k: KeyCode) -> u16 { use bevy::input::keyboard::KeyCode::*; match k { Space=>1, Digit1=>11, Digit2=>12, Digit3=>13, Digit4=>14, BracketLeft=>40, BracketRight=>41, KeyA=>100, KeyD=>101, KeyR=>102, ControlLeft=>200, F1=>250, _=> 1000 } }
fn mouse_rank(b: MouseButton) -> u16 { use MouseButton::*; match b { Left=>1, Right=>2, Middle=>3, Back=>4, Forward=>5, Other(x)=> 10 + x as u16 } }

fn parse_binding(spec: &str) -> Result<(SmallVec<[RawBindingToken;2]>, f32), String> { let mut tokens: SmallVec<[RawBindingToken;2]> = SmallVec::new(); let mut hold: f32 = 0.0; let mut seen: HashSet<RawBindingToken> = HashSet::new(); for part in spec.split('+') { let p = part.trim(); if p.is_empty() { continue; } if let Some(rest) = p.strip_prefix("hold>") { let mut it = rest.splitn(2, ':'); let secs_str = it.next().ok_or_else(|| format!("Malformed hold qualifier '{}': missing seconds", p))?; let after = it.next().ok_or_else(|| format!("Malformed hold qualifier '{}': missing ':'", p))?; hold = secs_str.parse::<f32>().map_err(|_| format!("Invalid hold seconds '{}'", secs_str))?; let token = parse_token(after)?; if !seen.insert(token) { return Err(format!("Duplicate token in chord: {:?}", token)); } tokens.push(token); continue; } let token = parse_token(p)?; if !seen.insert(token) { return Err(format!("Duplicate token in chord: {:?}", token)); } tokens.push(token); }
 if tokens.is_empty() { return Err("Empty binding".into()); }
 Ok((tokens, hold)) }

fn parse_single_token(s: &str) -> Result<RawBindingToken, String> { parse_token(s) }

fn parse_token(s: &str) -> Result<RawBindingToken, String> { if let Some(rest) = s.strip_prefix("Key:") { return parse_keycode(rest); } if let Some(rest) = s.strip_prefix("Mouse:") { return match rest { "Left" => Ok(RawBindingToken::MouseBtn(MouseButton::Left)), "Right" => Ok(RawBindingToken::MouseBtn(MouseButton::Right)), "Middle" => Ok(RawBindingToken::MouseBtn(MouseButton::Middle)), other => Err(format!("Unknown mouse button '{}'" , other)), }; } if let Some(rest) = s.strip_prefix("Wheel:") { return match rest { "Vertical" => Ok(RawBindingToken::WheelVert), "Horizontal" => Ok(RawBindingToken::WheelHorz), other => Err(format!("Unknown wheel axis '{}'" , other)), }; } if let Some(rest) = s.strip_prefix("MouseMove:") { return match rest { "X" => Ok(RawBindingToken::MouseMoveX), "Y" => Ok(RawBindingToken::MouseMoveY), other => Err(format!("Unknown mouse move axis '{}'" , other)), }; } if s == "Touch:Tap" { return Ok(RawBindingToken::TouchTap); } if s == "Touch:Drag" { return Ok(RawBindingToken::TouchDrag); } Err(format!("Unrecognized token '{}'" , s)) }

fn parse_keycode(name: &str) -> Result<RawBindingToken, String> { use bevy::input::keyboard::KeyCode; let kc = match name { "Space" => KeyCode::Space, "Digit1" => KeyCode::Digit1, "Digit2" => KeyCode::Digit2, "Digit3" => KeyCode::Digit3, "Digit4" => KeyCode::Digit4, "BracketLeft" => KeyCode::BracketLeft, "BracketRight" => KeyCode::BracketRight, "A"|"KeyA" => KeyCode::KeyA, "D"|"KeyD" => KeyCode::KeyD, "R"|"KeyR" => KeyCode::KeyR, "ControlLeft" => KeyCode::ControlLeft, "F1" => KeyCode::F1, other => return Err(format!("Unsupported KeyCode '{}' (extend parser)" , other)), }; Ok(RawBindingToken::Key(kc)) }
