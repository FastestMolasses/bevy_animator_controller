# Bevy Animator Controller Usage Guide

A Unity-like animator controller for Bevy, built on ozz-animation-rs.

## Quick Setup

```rust
use bevy::prelude::*;
use bevy_animator_controller::prelude::*;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, OzzAnimationPlugin))
        .run();
}
```

## Core Concepts

**AnimatorController**: Main component controlling animations, contains layers and parameters.
**AnimationLayer**: Organizes states and transitions within a blend hierarchy.
**AnimationState**: Simple (single animation) or Blend (blend tree).
**Parameters**: Bool, Float, Int, Trigger values controlling transitions and blending.
**Transitions**: Rules for changing between states based on parameter conditions.
**BlendTree**: Blends multiple animations based on parameter values (1D or 2D).

## Loading Assets

```rust
use ozz_animation_rs::*;

// Load skeleton
let skeleton = ozz_assets.get_mut(&skeleton_handle)?;
let skeleton = Arc::new(Skeleton::from_archive(&mut skeleton.archive)?);

// Load animation
let anim = ozz_assets.get_mut(&anim_handle)?;
let animation = Arc::new(Animation::from_archive(&mut anim.archive)?);
```

## Building an AnimatorController

### Simple Animation (Single State)

```rust
// Create layer
let mut layer = AnimationLayer::new(
    "Base Layer".to_string(),
    LayerBlendType::Override,
    1.0,                            // weight
    &skeleton,
    "idle".to_string(),             // default state
);

// Add state
let idle_state = SimpleState::new(animation, skeleton.num_soa_joints());
layer.add_state("idle".to_string(), AnimationState::Simple(idle_state));

// Create parameters
let mut parameters = Parameters::new();
parameters.set_float("speed", 0.0);

// Create controller
let controller = AnimatorController::new(
    skeleton.clone(),
    vec![layer],
    parameters,
);
```

## Parameters

```rust
let mut params = Parameters::new();

// Bool parameters
params.set_bool("is_running", true);
params.get_bool("is_running"); // Some(true)

// Float parameters
params.set_float("speed", 0.5);
params.get_float("speed"); // Some(0.5)

// Int parameters
params.set_int("state_id", 1);
params.get_int("state_id"); // Some(1)

// Triggers (auto-reset after update)
params.set_trigger("jump");
params.get_trigger("jump"); // true, then false after controller.update()
```

Update parameters on AnimatorController:
```rust
fn update_params(mut query: Query<&mut AnimatorController>) {
    for mut controller in query.iter_mut() {
        controller.parameters_mut().set_float("speed", 1.0);
    }
}
```

## Transitions

Define transitions between states with conditions:

```rust
layer.add_transition(
    "Idle".to_string(),
    Transition {
        to_state: "Run".to_string(),
        duration: 0.5,              // blend duration in seconds
        conditions: vec![
            TransitionCondition::Bool("is_running".to_string(), true),
        ],
        has_exit_time: false,
        exit_time: 0.0,
    },
);

// Transition back
layer.add_transition(
    "Run".to_string(),
    Transition {
        to_state: "Idle".to_string(),
        duration: 0.5,
        conditions: vec![
            TransitionCondition::Bool("is_running".to_string(), false),
        ],
        has_exit_time: false,
        exit_time: 0.0,
    },
);
```

### Transition Conditions

```rust
// Bool condition
TransitionCondition::Bool("is_running".to_string(), true)

// Float comparison
TransitionCondition::Float("speed".to_string(), 0.5, CompareType::Greater)
TransitionCondition::Float("speed".to_string(), 0.1, CompareType::Less)
TransitionCondition::Float("speed".to_string(), 1.0, CompareType::Equals)
TransitionCondition::Float("speed".to_string(), 0.0, CompareType::NotEqual)

// Int comparison
TransitionCondition::Int("state".to_string(), 2, CompareType::Greater)
TransitionCondition::Int("state".to_string(), 0, CompareType::Equals)

// Trigger
TransitionCondition::Trigger("jump".to_string())
```

## Blend Trees

### 1D Blend Tree

Blends between animations based on a single float parameter:

```rust
let idle_state = SimpleState::new(idle_anim, skeleton.num_soa_joints());
let run_state = SimpleState::new(run_anim, skeleton.num_soa_joints());

let motions = vec![
    MotionData {
        motion: BlendMotionState::Animation(Arc::new(RwLock::new(idle_state))),
        threshold: MotionThreshold::Simple1D(0.0),
    },
    MotionData {
        motion: BlendMotionState::Animation(Arc::new(RwLock::new(run_state))),
        threshold: MotionThreshold::Simple1D(1.0),
    },
];

let blend_tree = BlendTree::new(
    &skeleton,
    BlendTreeType::Simple1D("speed".to_string()),
    motions,
);

// Add as state
layer.add_state(
    "locomotion".to_string(),
    AnimationState::Blend(BlendState::new(blend_tree)),
);
```

### 2D Blend Tree

Blends based on two parameters (e.g., strafe movement):

```rust
let blend_tree = BlendTree::new(
    &skeleton,
    BlendTreeType::Directional2D("move_x".to_string(), "move_y".to_string()),
    vec![
        MotionData {
            motion: BlendMotionState::Animation(Arc::new(RwLock::new(forward_state))),
            threshold: MotionThreshold::Directional2D(0.0, 1.0),
        },
        MotionData {
            motion: BlendMotionState::Animation(Arc::new(RwLock::new(right_state))),
            threshold: MotionThreshold::Directional2D(1.0, 0.0),
        },
        MotionData {
            motion: BlendMotionState::Animation(Arc::new(RwLock::new(back_state))),
            threshold: MotionThreshold::Directional2D(0.0, -1.0),
        },
        MotionData {
            motion: BlendMotionState::Animation(Arc::new(RwLock::new(left_state))),
            threshold: MotionThreshold::Directional2D(-1.0, 0.0),
        },
    ],
);
```

### Nested Blend Trees

Blend trees can contain other blend trees:

```rust
let sub_tree = BlendTree::new(&skeleton, BlendTreeType::Simple1D("speed"), motions);
let sub_tree_state = BlendState::new(sub_tree);

let parent_motions = vec![
    MotionData {
        motion: BlendMotionState::SubTree(Arc::new(RwLock::new(sub_tree_state))),
        threshold: MotionThreshold::Simple1D(0.5),
    },
    // ... other motions
];
```

## Animation Layers

### Layer Blend Types

```rust
// Override: Replaces lower layers
LayerBlendType::Override

// Additive: Adds to lower layers
LayerBlendType::Additive
```

### Multiple Layers

```rust
let base_layer = AnimationLayer::new(
    "Base".to_string(),
    LayerBlendType::Override,
    1.0,
    &skeleton,
    "idle".to_string(),
);

let upper_body_layer = AnimationLayer::new(
    "UpperBody".to_string(),
    LayerBlendType::Additive,
    0.5,                            // 50% weight
    &skeleton,
    "wave".to_string(),
);

let controller = AnimatorController::new(
    skeleton.clone(),
    vec![base_layer, upper_body_layer],
    parameters,
);
```

### Dynamic Layer Weight

```rust
fn adjust_layer_weight(mut query: Query<&mut AnimatorController>) {
    for mut controller in query.iter_mut() {
        // Access layer by index
        if let Some(layer) = controller.layers_mut().get_mut(1) {
            layer.set_weight(0.75);
        }
    }
}
```

## Complete Example

```rust
fn build_controller(
    skeleton_handle: Handle<OzzAsset>,
    idle_handle: Handle<OzzAsset>,
    run_handle: Handle<OzzAsset>,
    mut ozz_assets: ResMut<Assets<OzzAsset>>,
) -> Option<AnimatorController> {
    // Load skeleton
    let skeleton = Arc::new(
        Skeleton::from_archive(&mut ozz_assets.get_mut(&skeleton_handle)?.archive).ok()?
    );
    
    // Load animations
    let idle_anim = Arc::new(
        Animation::from_archive(&mut ozz_assets.get_mut(&idle_handle)?.archive).ok()?
    );
    let run_anim = Arc::new(
        Animation::from_archive(&mut ozz_assets.get_mut(&run_handle)?.archive).ok()?
    );
    
    // Create layer
    let mut layer = AnimationLayer::new(
        "Base".to_string(),
        LayerBlendType::Override,
        1.0,
        &skeleton,
        "Idle".to_string(),
    );
    
    // Add states
    layer.add_state(
        "Idle".to_string(),
        AnimationState::Simple(SimpleState::new(idle_anim, skeleton.num_soa_joints())),
    );
    layer.add_state(
        "Run".to_string(),
        AnimationState::Simple(SimpleState::new(run_anim, skeleton.num_soa_joints())),
    );
    
    // Add transitions
    layer.add_transition("Idle".to_string(), Transition {
        to_state: "Run".to_string(),
        duration: 0.3,
        conditions: vec![TransitionCondition::Bool("is_running".to_string(), true)],
        has_exit_time: false,
        exit_time: 0.0,
    });
    
    layer.add_transition("Run".to_string(), Transition {
        to_state: "Idle".to_string(),
        duration: 0.3,
        conditions: vec![TransitionCondition::Bool("is_running".to_string(), false)],
        has_exit_time: false,
        exit_time: 0.0,
    });
    
    // Setup parameters
    let mut parameters = Parameters::new();
    parameters.set_bool("is_running", false);
    
    Some(AnimatorController::new(skeleton, vec![layer], parameters))
}
```

## Asset Loading Plugin

The library provides `OzzAssetPlugin` for loading .ozz files:

```rust
App::new()
    .add_plugins(OzzAssetPlugin::new(&["ozz"]))
    // ... rest of app
```

## System Integration

The plugin automatically adds these systems:
- `animate_bones`: Updates AnimatorController components
- `update_bone_transforms`: Applies transforms to skinned mesh bones
- `add_bone_indexes`: Initializes bone indices for new skinned meshes

These run in the `Update` and `PostUpdate` schedules respectively.
