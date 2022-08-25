use std::time::Duration;

use bevy::prelude::{shape::Quad, *};
use bevy::utils::{Entry, HashMap, HashSet};
use bevy_rapier3d::prelude::*;

use crate::game::animate::{AnimateRange, Ease};
use crate::game::camera::PlayerCamera;
use crate::game::progress_bar::{ProgressBar, ProgressBarBundle};

pub struct CardPlugin;

impl Plugin for CardPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SelectedCard>()
            .init_resource::<HoverPoint>()
            .init_resource::<StackRoots>()
            .add_system_to_stage(CoreStage::PostUpdate, on_spawn_card)
            .add_system(collide_cards)
            .add_system(
                select_card
                    .after(crate::game::camera::move_camera)
                    .after(collide_cards),
            )
            .add_system(move_cards.after(select_card))
            .add_system(evaluate_stacks.after(move_cards));
    }
}

#[derive(Component, Default)]
pub struct Card {
    pub animations: Animations,
    pub card_type: CardType,
    pub z: usize,
    pub stack_parent: Option<Entity>,
    pub stack_child: Option<Entity>,
}

impl Card {
    const ASPECT_RATIO: f32 = 50.0 / 60.0;
    const ART_WIDTH: f32 = 167.0;
    const ART_HEIGHT: f32 = 166.0;
    const ART_ASPECT: f32 = Self::ART_WIDTH / Self::ART_HEIGHT;
}

#[derive(Default, Copy, Clone, Hash, PartialEq, Eq, Debug)]
pub enum CardType {
    #[default]
    Villager,
}

impl CardType {
    pub fn class(self) -> CardClass {
        match self {
            CardType::Villager => CardClass::Character,
        }
    }

    pub fn image(self) -> &'static str {
        match self {
            CardType::Villager => "villager.png",
        }
    }
}

pub enum CardClass {
    Character,
}

impl CardClass {
    pub fn color(self) -> Color {
        match self {
            CardClass::Character => Color::rgb(0.4, 0.4, 0.4),
            // CardColor::Blue => Color::rgb(0.4, 0.4, 1.0),
            // CardColor::Yellow => Color::rgb(0.7, 0.7, 0.4),
        }
    }
}

#[derive(Default, PartialEq, Eq, Copy, Clone)]
pub enum SelectedCard {
    Some(Entity),
    #[default]
    None,
}

impl SelectedCard {
    fn is_selected(self, entity: Entity) -> bool {
        match self {
            SelectedCard::Some(e) => e == entity,
            SelectedCard::None => false,
        }
    }
}

#[derive(Default)]
pub enum HoverPoint {
    Some(Vec3),
    #[default]
    None,
}

#[derive(Bundle)]
pub struct CardBundle {
    pub card: Card,
    pub collider: Collider,
    pub sensor: Sensor,
    pub rigid_body: RigidBody,
    pub active_events: ActiveEvents,
    pub active_collision_types: ActiveCollisionTypes,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: Visibility,
    pub computed_visibiltiy: ComputedVisibility,
}

#[derive(Debug)]
pub enum StackType {
    Pending,
    Nothing,
    Breed { progress_bar: Entity },
}

#[derive(Default)]
pub struct StackRoots {
    roots: HashMap<Entity, StackType>,
    queued_stack_recomputations: HashSet<Entity>,
}

impl Default for CardBundle {
    fn default() -> Self {
        Self {
            collider: Collider::cuboid(Card::ASPECT_RATIO / 2.0, 1.0 / 2.0, 0.2),
            sensor: Sensor,
            active_events: ActiveEvents::COLLISION_EVENTS,
            active_collision_types: ActiveCollisionTypes::all(),
            rigid_body: RigidBody::Fixed,
            card: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
            visibility: Default::default(),
            computed_visibiltiy: Default::default(),
        }
    }
}

fn on_spawn_card(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    cards: Query<(Entity, &Card), Added<Card>>,
) {
    for (entity, card) in &cards {
        commands.entity(entity).with_children(|parent| {
            parent.spawn_bundle(PbrBundle {
                material: materials.add(StandardMaterial {
                    base_color: card.card_type.class().color(),
                    base_color_texture: Some(asset_server.load("card_base.png")),
                    alpha_mode: AlphaMode::Blend,
                    unlit: true,
                    ..default()
                }),
                mesh: meshes.add(
                    Quad {
                        size: Vec2::new(Card::ASPECT_RATIO, 1.0),
                        ..default()
                    }
                    .into(),
                ),
                ..default()
            });
            parent.spawn_bundle(PbrBundle {
                material: materials.add(StandardMaterial {
                    base_color: card.card_type.class().color(),
                    base_color_texture: Some(asset_server.load(card.card_type.image())),
                    alpha_mode: AlphaMode::Blend,
                    unlit: true,
                    ..default()
                }),
                mesh: meshes.add(
                    Quad {
                        size: Vec2::new(Card::ART_ASPECT, 1.0) * 0.65,
                        ..default()
                    }
                    .into(),
                ),
                transform: Transform::from_xyz(0.0, -0.08, 0.001),
                ..default()
            });
        });
    }
}

fn move_cards(
    time: Res<Time>,
    selected: Res<SelectedCard>,
    hover_point: Res<HoverPoint>,
    stack_roots: Res<StackRoots>,
    mut cards: Query<(Entity, &mut Card, &mut Transform)>,
) {
    for (entity, mut card, mut transform) in &mut cards {
        let mut z_offset = 0.0;
        if selected.is_selected(entity) {
            z_offset += card.animations.select.tick(time.delta());
            if let HoverPoint::Some(hover_point) = *hover_point {
                transform.translation.x = hover_point.x;
                transform.translation.y = hover_point.y;
            }
        } else {
            z_offset += card.animations.deselect.tick(time.delta());
        }
        transform.translation.z = z_offset;
    }

    for root in stack_roots.roots.keys() {
        let result = cards
            .get(*root)
            .ok()
            .and_then(|(_, card, transform)| card.stack_child.map(|e| (e, transform.translation)));
        if let Some((child, translation)) = result {
            position_stack(&mut cards, child, translation, 1);
        }
    }
}

fn position_stack(
    cards: &mut Query<(Entity, &mut Card, &mut Transform)>,
    entity: Entity,
    root_position: Vec3,
    depth: usize,
) {
    let child = if let Ok((_, card, mut transform)) = cards.get_mut(entity) {
        transform.translation =
            root_position + Vec3::new(0.0, -0.3 * depth as f32, 0.01 * depth as f32);
        card.stack_child
    } else {
        None
    };

    if let Some(child) = child {
        position_stack(cards, child, root_position, depth + 1);
    }
}

fn collide_cards(
    mut commands: Commands,
    mut collisions: EventReader<CollisionEvent>,
    mut stack_roots: ResMut<StackRoots>,
    mut selected: Res<SelectedCard>,
    mut cards: Query<&mut Card>,
    transforms: Query<&Transform>,
) {
    let mut stack_x_on_y = Vec::new();
    for collision in collisions.iter() {
        match *collision {
            CollisionEvent::Started(e1, e2, _) => {
                if selected.is_selected(e1) || selected.is_selected(e2) {
                    continue;
                }
                if let (Ok([mut c1, mut c2]), Ok([t1, t2])) =
                    (cards.get_many_mut([e1, e2]), transforms.get_many([e1, e2]))
                {
                    if t1.translation.z > t2.translation.z {
                        if c1.stack_parent.is_none() {
                            stack_x_on_y.push((e1, e2));
                        }
                    } else {
                        if c2.stack_parent.is_none() {
                            stack_x_on_y.push((e2, e1));
                        }
                    }
                }
            }
            _ => {}
        }
    }

    for (ex, ey) in stack_x_on_y {
        let top = find_stack_top(&cards.to_readonly(), ey);
        if let Ok([mut cx, mut ctop]) = cards.get_many_mut([ex, top]) {
            if cx.stack_parent.is_none() && ctop.stack_child.is_none() {
                // update pointers
                ctop.stack_child = Some(ex);
                cx.stack_parent = Some(top);

                match stack_roots.roots.entry(top) {
                    // if stack root is already a stack, queue recalculation
                    Entry::Occupied(_) => {
                        stack_roots.queued_stack_recomputations.insert(top);
                    }
                    // if parent is newly stacked, make it a stack root and recompute
                    Entry::Vacant(mut entry) => {
                        entry.insert(StackType::Pending);
                        stack_roots.queued_stack_recomputations.insert(top);
                    }
                }

                match stack_roots.roots.entry(ex) {
                    // if newly stacked card is a stack, queue it for recomputation (and therefore removal)
                    Entry::Occupied(_) => {
                        stack_roots.queued_stack_recomputations.insert(ex);
                    }
                    // if newly stacked card is not a stack, do nothing
                    Entry::Vacant(_) => {}
                }
            }
        }
    }
}

fn find_stack_top(cards: &Query<&Card>, mut current_entity: Entity) -> Entity {
    loop {
        if let Ok(card) = cards.get(current_entity) {
            if let Some(child) = card.stack_child {
                current_entity = child;
            } else {
                return current_entity;
            }
        } else {
            return current_entity;
        }
    }
}

fn find_stack_root(cards: &Query<&Card>, mut current_entity: Entity) -> Entity {
    loop {
        if let Ok(card) = cards.get(current_entity) {
            if let Some(parent) = card.stack_parent {
                current_entity = parent;
            } else {
                return current_entity;
            }
        } else {
            return current_entity;
        }
    }
}

fn select_card(
    context: Res<RapierContext>,
    windows: Res<Windows>,
    mut selected_card: ResMut<SelectedCard>,
    mut hover_point: ResMut<HoverPoint>,
    mut stack_roots: ResMut<StackRoots>,
    mouse: Res<Input<MouseButton>>,
    mut cards: Query<&mut Card>,
    cameras: Query<(&Camera, &Transform), With<PlayerCamera>>,
) {
    let window = windows.primary();
    if let Some(mut cursor) = window.cursor_position() {
        let (camera, camera_transform) = cameras.single();

        let view = camera_transform.compute_matrix();

        let (viewport_min, viewport_max) = camera.logical_viewport_rect().unwrap();
        let screen_size = camera.logical_target_size().unwrap();
        let viewport_size = viewport_max - viewport_min;
        let adj_cursor_pos = cursor - Vec2::new(viewport_min.x, screen_size.y - viewport_max.y);
        let projection = camera.projection_matrix();
        let far_ndc = projection.project_point3(Vec3::NEG_Z).z;
        let near_ndc = projection.project_point3(Vec3::Z).z;
        let cursor_ndc = (adj_cursor_pos / viewport_size) * 2.0 - Vec2::ONE;
        let ndc_to_world: Mat4 = view * projection.inverse();
        let near = ndc_to_world.project_point3(cursor_ndc.extend(near_ndc));
        let far = ndc_to_world.project_point3(cursor_ndc.extend(far_ndc));
        let direction = far - near;

        let denom = Vec3::Z.dot(direction);
        if denom.abs() > 0.0001 {
            let t = (Vec3::ZERO - near).dot(Vec3::Z) / denom;
            if t >= 0.0 {
                *hover_point = HoverPoint::Some(near + direction * t);
            } else {
                *hover_point = HoverPoint::None;
            }
        } else {
            *hover_point = HoverPoint::None;
        }

        if mouse.just_pressed(MouseButton::Left) {
            let result = context.cast_ray(near, direction, 50.0, true, QueryFilter::new());

            if let Some((entity, toi)) = result {
                let (parent, child) = {
                    let mut card = cards.get_mut(entity).unwrap();
                    card.animations.select.reset();
                    *selected_card = SelectedCard::Some(entity);
                    let parent = card.stack_parent;
                    card.stack_parent = None;
                    (parent, card.stack_child)
                };
                // finish unstack
                if let Some(parent) = parent {
                    let mut card = cards.get_mut(parent).unwrap();
                    card.stack_child = None;
                    // queue parent for recomputation
                    stack_roots.queued_stack_recomputations.insert(parent);

                    // unstacked card is now a stack root, create a new stack root as pending and recompute
                    if child.is_some() {
                        stack_roots.roots.insert(entity, StackType::Pending);
                        stack_roots.queued_stack_recomputations.insert(entity);
                    }
                }
            }
        }
    }

    if mouse.just_released(MouseButton::Left) {
        if let SelectedCard::Some(entity) = *selected_card {
            let mut card = cards.get_mut(entity).unwrap();
            card.animations.deselect.reset();
            *selected_card = SelectedCard::None;
        }
    }
}

fn evaluate_stacks(
    mut commands: Commands,
    time: Res<Time>,
    mut stack_roots: ResMut<StackRoots>,
    cards: Query<&Card>,
    mut progress_bars: Query<&mut ProgressBar>,
    transforms: Query<&Transform>,
) {
    let stack_roots = &mut *stack_roots;
    for entity in stack_roots.queued_stack_recomputations.drain() {
        let root = find_stack_root(&cards, entity);
        let mut cancelled_stack_types = Vec::new();
        if root != entity {
            // if the queued entity is no longer a root, remove the root and cancel the current stack_type
            if let Some(stack_type) = stack_roots.roots.remove(&entity) {
                cancelled_stack_types.push(stack_type);
            }
        }
        // if the queued root is still a root, recompute the stack type
        let card_types = get_cards_types(root, &cards);
        let villagers = card_types.get(&CardType::Villager).unwrap_or(&0);
        let new_stack_type = if *villagers == 2 && card_types.len() == 1 {
            let mut progress_bar = None;
            commands.entity(root).with_children(|parent| {
                progress_bar = Some(
                    parent
                        .spawn_bundle(ProgressBarBundle {
                            progress_bar: ProgressBar {
                                current: 0.0,
                                total: 5.0,
                                width: 0.7,
                                height: 0.15,
                                padding: 0.05,
                            },
                            transform: Transform::from_xyz(0.0, 0.55, 0.0),
                            ..default()
                        })
                        .id(),
                );
            });
            StackType::Breed {
                progress_bar: progress_bar.unwrap(),
            }
        } else {
            StackType::Nothing
        };

        // insert the new stack type and cancel the old one, if it exists
        if let Some(stack_type) = stack_roots.roots.insert(root, new_stack_type) {
            cancelled_stack_types.push(stack_type);
        }

        for stack_type in cancelled_stack_types {
            match stack_type {
                StackType::Pending => {}
                StackType::Nothing => {}
                StackType::Breed { progress_bar } => {
                    commands.entity(progress_bar).despawn_recursive();
                }
            }
        }
    }

    let mut queued_recomputations = Vec::new();
    for (root, stack_type) in stack_roots.roots.iter_mut() {
        let mut should_reset = false;
        match stack_type {
            StackType::Pending => {}
            StackType::Nothing => {}
            StackType::Breed { progress_bar } => {
                if let Ok(mut bar) = progress_bars.get_mut(*progress_bar) {
                    bar.add(time.delta_seconds());
                    if bar.finished() {
                        commands.entity(*progress_bar).despawn_recursive();
                        if let Ok(transform) = transforms.get(*root) {
                            commands.spawn_bundle(CardBundle {
                                card: Card {
                                    card_type: CardType::Villager,
                                    ..default()
                                },
                                transform: Transform::from_xyz(
                                    transform.translation.x + 1.0,
                                    transform.translation.y + 1.0,
                                    0.0,
                                ),
                                ..default()
                            });
                        }
                        should_reset = true;
                    }
                }
            }
        }
        if should_reset {
            *stack_type = StackType::Pending;
            queued_recomputations.push(*root);
        }
    }

    stack_roots
        .queued_stack_recomputations
        .extend(queued_recomputations);
}

fn get_cards_types(root: Entity, cards: &Query<&Card>) -> HashMap<CardType, usize> {
    let mut current = root;
    let mut card_types = HashMap::new();
    while let Ok(card) = cards.get(current) {
        let mut count = card_types.entry(card.card_type).or_insert(0);
        *count += 1;
        if let Some(child) = card.stack_child {
            current = child;
        } else {
            break;
        }
    }

    card_types
}

pub struct Animations {
    select: AnimateRange,
    deselect: AnimateRange,
}

impl Default for Animations {
    fn default() -> Self {
        Self {
            select: AnimateRange::new(Duration::from_secs_f32(0.2), Ease::Linear, 0.0..0.5, false),
            deselect: AnimateRange::new(
                Duration::from_secs_f32(0.2),
                Ease::Linear,
                0.5..0.0,
                false,
            ),
        }
    }
}
