use macroquad::prelude::*;
use shipyard::{
    AddComponent, AllStoragesViewMut, EntitiesViewMut, IntoIter, IntoWithId, SparseSet, UniqueView,
    UniqueViewMut, View, ViewMut, Workload, World,
};

const WIDTH: i32 = 640;
const HEIGHT: i32 = 360;
const INIT_SIZE: f32 = 5.;
const MAX_SIZE: f32 = 25.;
const GROWTH_RATE: f32 = 0.15;
const SPEED: f32 = 1.5;
const ACCELERATION_RATE: f32 = 0.01;
const SQUARE_SPAWN_RATE: u32 = 25;
const SQUAGUM_SPAWN_RATE: u32 = 150;

struct Player {
    is_invincible: bool,
    i_counter: u32,
    squagum: bool,
    squagum_counter: u32,
    rect: Rect,
}

struct Squagum(Vec2);
struct Acceleration(f32);
struct ToDelete;

#[derive(Debug)]
enum GameOver {
    Loose,
    Victory,
}

impl std::error::Error for GameOver {}

impl std::fmt::Display for GameOver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

/// generates a new random square.
fn new_square() -> (Rect, Acceleration) {
    (
        Rect {
            x: rand::gen_range(MAX_SIZE / 2.0, WIDTH as f32 - MAX_SIZE / 2.),
            y: rand::gen_range(MAX_SIZE / 2.0, HEIGHT as f32 - MAX_SIZE / 2.),
            w: INIT_SIZE,
            h: INIT_SIZE,
        },
        Acceleration(0.),
    )
}

fn window_conf() -> Conf {
    Conf {
        window_title: "Square Eater".to_owned(),
        window_width: WIDTH,
        window_height: HEIGHT,
        ..Default::default()
    }
}

fn init_world(world: &mut World) {
    let _ = world.remove_unique::<Player>();

    world
        .add_unique(Player {
            is_invincible: false,
            i_counter: 0,
            squagum: false,
            squagum_counter: 0,
            rect: Rect::new(0., 0., INIT_SIZE * 3., INIT_SIZE * 3.),
        })
        .unwrap();

    world.bulk_add_entity((0..7).map(|_| new_square()));
}

// Entry point of the program
#[macroquad::main(window_conf)]
async fn main() {
    let mut world = World::new();

    init_world(&mut world);

    // seed the random number generator with a random value
    rand::srand(macroquad::miniquad::date::now() as u64);

    Workload::builder("Game loop")
        .with_system(&counters)
        .with_system(&move_player)
        .with_system(&move_square)
        .with_system(&grow_square)
        .with_system(&new_squares)
        .with_system(&collision)
        .with_try_system(&clean_up)
        .with_system(&render)
        .add_to_world(&world)
        .unwrap();

    let mut is_started = false;
    loop {
        if is_started {
            clear_background(WHITE);

            if let Err(Some(err)) = world
                .run_default()
                .map_err(shipyard::error::RunWorkload::custom_error)
            {
                match err.downcast_ref::<GameOver>().unwrap() {
                    GameOver::Loose => debug!("GameOver"),
                    GameOver::Victory => debug!("Victory"),
                }

                is_started = false;
                world.clear();
                init_world(&mut world);
            }
        } else {
            if is_mouse_button_pressed(MouseButton::Left) {
                is_started = true;

                unsafe {
                    get_internal_gl().quad_context.show_mouse(false);
                }
            }

            clear_background(BLACK);

            let text_dimensions = measure_text("Click to start", None, 40, 1.);
            draw_text(
                "Click to start",
                WIDTH as f32 / 2. - text_dimensions.0 / 2.,
                HEIGHT as f32 / 2. - text_dimensions.1 / 2.,
                40.,
                WHITE,
            );
        }

        next_frame().await
    }
}

fn counters(mut player: UniqueViewMut<Player>) {
    if player.is_invincible {
        player.i_counter += 1;

        if player.i_counter >= 10 {
            player.is_invincible = false;
            player.i_counter = 0;
        }
    }

    if player.squagum {
        player.squagum_counter += 1;

        if player.squagum_counter >= 120 {
            player.squagum = false;
            player.squagum_counter = 0;
        }
    }
}

fn move_player(mut player: UniqueViewMut<Player>) {
    let (x, y) = mouse_position();
    player.rect.x = x.clamp(player.rect.w / 2., WIDTH as f32 - player.rect.w / 2.);
    player.rect.y = y.clamp(player.rect.h / 2., HEIGHT as f32 - player.rect.h / 2.);
}

fn move_square(
    player: UniqueView<Player>,
    mut rects: ViewMut<Rect>,
    mut accelerations: ViewMut<Acceleration>,
) {
    for mut acceleration in (&mut accelerations).iter() {
        acceleration.0 += ACCELERATION_RATE;
    }

    let mut dirs = vec![Vec2::zero(); rects.len()];

    for ((id, rect), dir) in rects.iter().with_id().zip(&mut dirs) {
        if rect.w > player.rect.w && rect.h > player.rect.h {
            let player_dir = player.rect.point()
                - Vec2::new(player.rect.w / 2., player.rect.h / 2.)
                - Vec2::new(rect.x - rect.w / 2., rect.y - rect.h / 2.);

            *dir = player_dir.normalize();

            if player.squagum {
                *dir = -*dir;
            }

            let mut neighbourg_dir = Vec2::zero();

            for neighbourg in rects.iter() {
                if rect.point().distance_squared(neighbourg.point()) < rect.w * rect.h / 1.5 {
                    neighbourg_dir += Vec2::new(rect.x - neighbourg.x, rect.y - neighbourg.y);
                }
            }

            if rect.w == MAX_SIZE && rect.h == MAX_SIZE {
                *dir *= SPEED + accelerations[id].0;
            } else {
                *dir *= SPEED;
            }

            *dir += rect.point() + neighbourg_dir * 0.05;

            dir.x = dir.x.clamp(INIT_SIZE / 2., WIDTH as f32 - INIT_SIZE / 2.);
            dir.y = dir.y.clamp(INIT_SIZE / 2., HEIGHT as f32 - INIT_SIZE / 2.);
        }
    }

    for (mut rect, dir) in (&mut rects).iter().zip(dirs) {
        if dir != Vec2::zero() {
            rect.move_to(dir);
        }
    }
}

fn grow_square(mut rects: ViewMut<Rect>) {
    for mut rect in (&mut rects).iter() {
        rect.w = (rect.w + GROWTH_RATE).min(MAX_SIZE);
        rect.h = (rect.h + GROWTH_RATE).min(MAX_SIZE);
    }
}

fn new_squares(
    mut entities: EntitiesViewMut,
    mut rects: ViewMut<Rect>,
    mut accelerations: ViewMut<Acceleration>,
    mut squagums: ViewMut<Squagum>,
) {
    if rand::gen_range(0, SQUARE_SPAWN_RATE) == 0 {
        entities.add_entity((&mut rects, &mut accelerations), new_square());
    }

    if rand::gen_range(0, SQUAGUM_SPAWN_RATE) == 0 {
        entities.add_entity(
            &mut squagums,
            Squagum(Vec2::new(
                rand::gen_range(0.0, WIDTH as f32),
                rand::gen_range(0.0, HEIGHT as f32),
            )),
        );
    }
}

fn collision(
    mut player: UniqueViewMut<Player>,
    rects: View<Rect>,
    squagums: View<Squagum>,
    mut to_delete: ViewMut<ToDelete>,
) {
    for (id, squagum) in squagums.iter().with_id() {
        if player.rect.contains(squagum.0)
            || player
                .rect
                .contains(squagum.0 + Vec2::new(INIT_SIZE, INIT_SIZE))
        {
            player.squagum = true;
            to_delete.add_component_unchecked(id, ToDelete);
        }
    }

    for (id, rect) in rects.iter().with_id() {
        if rect.w == MAX_SIZE
            && rect.h == MAX_SIZE
            && rect.x - rect.w / 2. <= player.rect.x + player.rect.w / 2.
            && rect.x + rect.w / 2. >= player.rect.x - player.rect.w / 2.
            && rect.y - rect.h / 2. <= player.rect.y + player.rect.h / 2.
            && rect.y + rect.h / 2. >= player.rect.y - player.rect.h / 2.
        {
            if player.squagum {
                player.rect.w = (player.rect.w + INIT_SIZE / 4.).min(MAX_SIZE - 0.01);
                player.rect.h = (player.rect.h + INIT_SIZE / 4.).min(MAX_SIZE - 0.01);
                to_delete.add_component_unchecked(id, ToDelete);
            }

            if !player.is_invincible {
                player.is_invincible = true;
                player.rect.w -= INIT_SIZE / 2.;
                player.rect.h -= INIT_SIZE / 2.;
            }
        } else if player.rect.x >= rect.w
            && player.rect.h >= rect.h
            && player.rect.x - player.rect.w / 2. <= rect.x + rect.w / 2.
            && player.rect.x + player.rect.w / 2. >= rect.x - rect.w / 2.
            && player.rect.y - player.rect.h / 2. <= rect.y + rect.h / 2.
            && player.rect.y + player.rect.h / 2. >= rect.y - rect.h / 2.
        {
            player.rect.w = (player.rect.w + INIT_SIZE / 2.).min(MAX_SIZE - 0.01);
            player.rect.h = (player.rect.h + INIT_SIZE / 2.).min(MAX_SIZE - 0.01);
            to_delete.add_component_unchecked(id, ToDelete)
        }
    }
}

fn clean_up(mut all_storages: AllStoragesViewMut) -> Result<(), GameOver> {
    all_storages.delete_any::<SparseSet<ToDelete>>();

    let (player, rects) = all_storages
        .borrow::<(UniqueView<Player>, View<Rect>)>()
        .unwrap();

    if player.rect.w < INIT_SIZE || player.rect.h < INIT_SIZE {
        Err(GameOver::Loose)
    } else {
        if rects.is_empty() {
            Err(GameOver::Victory)
        } else {
            Ok(())
        }
    }
}

fn render(player: UniqueView<Player>, rects: View<Rect>, squagums: View<Squagum>) {
    for rect in rects.iter() {
        draw_rectangle(
            rect.x - rect.w / 2.,
            rect.y - rect.h / 2.,
            rect.w,
            rect.h,
            if rect.h == MAX_SIZE && rect.w == MAX_SIZE {
                RED
            } else if rect.w > player.rect.w && rect.h > player.rect.h {
                GRAY
            } else {
                GREEN
            },
        );
    }

    for squagum in squagums.iter() {
        draw_rectangle(squagum.0.x, squagum.0.y, INIT_SIZE, INIT_SIZE, YELLOW);
    }

    draw_rectangle(
        player.rect.x - player.rect.w / 2.,
        player.rect.y - player.rect.h / 2.,
        player.rect.w,
        player.rect.h,
        if !player.squagum { BLUE } else { YELLOW },
    );
}
