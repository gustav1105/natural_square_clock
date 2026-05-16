use scz::{NaturalSquaresEngine, SpiralIterator};
use skrifa::{FontRef, MetadataProvider};
use std::sync::Arc;
use vello::kurbo::{Affine, BezPath, Circle, Line, Point, Rect, Stroke};
use vello::peniko::Color;
use vello::util::RenderContext;
use vello::{AaConfig, Glyph, Renderer, RendererOptions, Scene};
use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::WindowBuilder,
};

fn point_on_circle(center: Point, radius: f64, angle_deg: f64) -> Point {
    let rad = angle_deg.to_radians();

    Point::new(center.x + radius * rad.cos(), center.y - radius * rad.sin())
}

async fn run() {
    let event_loop = EventLoop::new().unwrap();

    let window = Arc::new(
        WindowBuilder::new()
            .with_title("Natural Squares Clock")
            .with_inner_size(winit::dpi::LogicalSize::new(1000.0, 1000.0))
            .build(&event_loop)
            .unwrap(),
    );

    // =====================================================
    // NUMBER FONT
    // =====================================================
    let mono_font_data = include_bytes!("../NotoSansSymbols2-Regular.ttf");
    let mono_font = vello::peniko::Font::new(mono_font_data.to_vec().into(), 0);
    let mono_font_ref = FontRef::new(mono_font_data).expect("Failed mono font");
    let mono_charmap = mono_font_ref.charmap();

    // =====================================================
    // SYMBOL FONT
    // =====================================================
    let symbol_font_data = include_bytes!("../NotoEmoji-Bold.ttf");
    let symbol_font = vello::peniko::Font::new(symbol_font_data.to_vec().into(), 0);
    let symbol_font_ref = FontRef::new(symbol_font_data).expect("Failed symbol font");
    let symbol_charmap = symbol_font_ref.charmap();

    // =====================================================
    // GPU
    // =====================================================
    let mut context = RenderContext::new();
    let size = window.inner_size();

    let surface = context
        .create_surface(
            window.clone(),
            size.width,
            size.height,
            vello::wgpu::PresentMode::Fifo,
        )
        .await
        .unwrap();

    let device_handle = &context.devices[surface.dev_id];

    let mut renderer = Renderer::new(
        &device_handle.device,
        RendererOptions {
            surface_format: Some(surface.format),
            use_cpu: false,
            antialiasing_support: vello::AaSupport::all(),
            num_init_threads: None,
        },
    )
    .unwrap();

    let mut scene = Scene::new();

    let grid_points: Vec<(u32, i32, i32)> = SpiralIterator::default().take(361).collect();
    let month_lengths = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

    let mut month_starts = [0u32; 12];
    let mut acc = 0;
    for (i, len) in month_lengths.iter().enumerate() {
        month_starts[i] = acc;
        acc += len;
    }

    event_loop
        .run(move |event, event_loop_target| {
            match event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    event_loop_target.exit();
                }

                Event::AboutToWait => {
                    // Request a frame redraw
                    window.request_redraw();

                    // Efficient Timing: Calculate when the next exact second happens
                    // and tell the event loop to sleep until then.
                    let now = std::time::Instant::now();
                    let current_ms = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis()
                        % 1000;

                    let time_to_next_second =
                        std::time::Duration::from_millis((1000 - current_ms) as u64);
                    event_loop_target.set_control_flow(winit::event_loop::ControlFlow::WaitUntil(
                        now + time_to_next_second,
                    ));
                }

                Event::WindowEvent {
                    event: WindowEvent::RedrawRequested,
                    ..
                } => {
                    let width = window.inner_size().width;
                    let height = window.inner_size().height;

                    let Ok(surface_texture) = surface.surface.get_current_texture() else {
                        return;
                    };

                    let render_params = vello::RenderParams {
                        base_color: Color::rgb8(10, 12, 15),
                        width,
                        height,
                        antialiasing_method: AaConfig::Msaa16,
                    };

                    scene.reset();

                    let center_x = width as f64 / 2.0;
                    let center_y = height as f64 / 2.0;
                    let center = Point::new(center_x, center_y);
                    let scale = 30.0;

                    // =====================================================
                    // SPIRAL GRID
                    // =====================================================
                    for (val, x, y) in &grid_points {
                        let px = center_x + (*x as f64 * scale);
                        let py = center_y - (*y as f64 * scale);

                        let rect = Rect::new(
                            px - scale / 2.0,
                            py - scale / 2.0,
                            px + scale / 2.0,
                            py + scale / 2.0,
                        );

                        scene.stroke(
                            &Stroke::new(0.5),
                            Affine::IDENTITY,
                            Color::rgb8(55, 55, 65),
                            None,
                            &rect,
                        );

                        // Number Text
                        let text = val.to_string();
                        let mut x_cursor = px - ((text.len() as f64) * 5.0);
                        let y_cursor = py + 5.0;

                        let glyphs: Vec<Glyph> = text
                            .chars()
                            .map(|c| {
                                let gid = mono_charmap.map(c).unwrap_or_default();
                                let g = Glyph {
                                    id: gid.to_u32(),
                                    x: x_cursor as f32,
                                    y: y_cursor as f32,
                                };
                                x_cursor += 8.0;
                                g
                            })
                            .collect();

                        scene
                            .draw_glyphs(&mono_font)
                            .font_size(13.0)
                            .brush(&Color::WHITE)
                            .draw(vello::peniko::Fill::NonZero, glyphs.into_iter());
                    }

                    // =====================================================
                    // OUTER RING
                    // =====================================================
                    let outer_radius = 560.0;
                    let outer_circle = Circle::new(center, outer_radius);

                    scene.stroke(
                        &Stroke::new(2.0),
                        Affine::IDENTITY,
                        Color::rgb8(0, 220, 255),
                        None,
                        &outer_circle,
                    );

                    // =====================================================
                    // YEAR / DATE RING (Calculations)
                    // =====================================================
                    use chrono::{Datelike, Local, Timelike};

                    let today_resolved = Local::now();
                    let today_day_of_year = (today_resolved.ordinal() - 1) as i32;

                    // 1. Calculate today's exact base calendar angle
                    let today_angle =
                        NaturalSquaresEngine::day_of_year_to_angle(today_day_of_year as f32, true)
                            as f64;

                    // 2. Calculate how many degrees into the day we are (Now down to the second!)
                    let current_hour = today_resolved.hour() as f64;
                    let current_minute = today_resolved.minute() as f64;
                    let current_second = today_resolved.second() as f64;

                    // 1 hour = 15°, 1 minute = 0.25°, 1 second = 0.004166...°
                    let time_offset_deg = (current_hour * 15.0)
                        + (current_minute * 0.25)
                        + (current_second * (15.0 / 3600.0));

                    // 3. Align the current time position exactly over today's date marker
                    let clock_rotation_deg = today_angle - time_offset_deg;

                    // =====================================================
                    // 360° TICKS
                    // =====================================================
                    for deg in 0..360 {
                        let angle = deg as f64;
                        let outer = point_on_circle(center, outer_radius, angle);

                        let tick_len = if deg % 90 == 0 { 20.0 } else { 0.0 };

                        let inner = point_on_circle(center, outer_radius + tick_len, angle);
                        let line = Line::new(inner, outer);

                        scene.stroke(
                            &Stroke::new(1.0),
                            Affine::IDENTITY,
                            Color::rgb8(0, 220, 255),
                            None,
                            &line,
                        );
                    }

                    // =====================================================
                    // 24 HOUR DIVIDERS
                    // =====================================================
                    for hour in 0..24 {
                        let angle = (hour as f64 * 15.0) + clock_rotation_deg;

                        let inner = point_on_circle(center, outer_radius - 70.0, angle);
                        let outer = point_on_circle(center, outer_radius, angle);

                        scene.stroke(
                            &Stroke::new(1.2),
                            Affine::IDENTITY,
                            Color::rgb8(0, 140, 180),
                            None,
                            &Line::new(inner, outer),
                        );
                    }

                    // =====================================================
                    // 24 HOUR RING (Numbers)
                    // =====================================================
                    for hour in 0..=23 {
                        let angle = (hour as f64 * 15.0) + clock_rotation_deg;

                        let p = point_on_circle(center, outer_radius - 40.0, angle);
                        let text = hour.to_string();

                        let mut x_cursor = p.x - ((text.len() as f64) * 4.5);
                        let y_cursor = p.y + 5.0;

                        let glyphs: Vec<Glyph> = text
                            .chars()
                            .map(|c| {
                                let gid = mono_charmap.map(c).unwrap_or_default();
                                let g = Glyph {
                                    id: gid.to_u32(),
                                    x: x_cursor as f32,
                                    y: y_cursor as f32,
                                };
                                x_cursor += 8.0;
                                g
                            })
                            .collect();

                        scene
                            .draw_glyphs(&mono_font)
                            .font_size(16.0)
                            .brush(&Color::rgb8(120, 220, 255))
                            .draw(vello::peniko::Fill::NonZero, glyphs.into_iter());
                    }

                    // =====================================================
                    // YEAR / DATE RING DRAWING
                    // =====================================================
                    let year_radius = outer_radius - 90.0;

                    for day in 0..365 {
                        let is_month_start = month_starts.contains(&(day as u32));
                        let is_today = day == today_day_of_year;

                        let angle =
                            NaturalSquaresEngine::day_of_year_to_angle(day as f32, true) as f64;

                        let outer = point_on_circle(center, year_radius, angle);

                        let tick_len = if is_today {
                            18.0
                        } else if is_month_start {
                            14.0
                        } else {
                            6.0
                        };

                        let inner = point_on_circle(center, year_radius - tick_len, angle);

                        let color = if is_today {
                            Color::rgb8(255, 0, 0)
                        } else if is_month_start {
                            Color::rgb8(255, 140, 0)
                        } else {
                            Color::rgb8(90, 90, 100)
                        };

                        let stroke_width = if is_today { 1.5 } else { 1.0 };

                        scene.stroke(
                            &Stroke::new(stroke_width),
                            Affine::IDENTITY,
                            color,
                            None,
                            &Line::new(inner, outer),
                        );
                    }

                    // =====================================================
                    // STATIC 13-SIGN MASTER ZODIAC (Aries Hard-Locked to 0° East)
                    // =====================================================

                    let zodiac_symbols = [
                        "♈", "♉", "♊", "♋", "♌", "♍", "♎", "♏", "⛎", "♐", "♑", "♒",
                        "♓",
                    ];

                    for index in 0..13 {
                        let symbol = zodiac_symbols[index];

                        // Query your engine directly for the pure structural coordinate angle
                        let base_angle = NaturalSquaresEngine::zodiac_to_angle(index as u32) as f64;
                        let final_angle = base_angle % 360.0;

                        // -------------------------------------------------
                        // DRAW GOLDEN AXIS LINES (Outer Edge Straight to Center)
                        // -------------------------------------------------
                        let tick_start = point_on_circle(center, outer_radius, final_angle);

                        // Point the line directly into the absolute center point
                        let tick_end = center;

                        let mut path = vello::kurbo::BezPath::new();
                        path.move_to(vello::kurbo::Point::new(tick_start.x, tick_start.y));
                        path.line_to(vello::kurbo::Point::new(tick_end.x, tick_end.y));

                        scene.stroke(
                            &vello::kurbo::Stroke::new(1.0),
                            vello::kurbo::Affine::IDENTITY,
                            &Color::rgb8(255, 140, 0), // Fucking golden lines
                            None,
                            &path,
                        );

                        // -------------------------------------------------
                        // DRAW GLYPH LABELS
                        // -------------------------------------------------
                        let p = point_on_circle(center, outer_radius + 45.0, final_angle);
                        let mut x_cursor = p.x - 12.0;
                        let y_cursor = p.y + 10.0;

                        let glyphs: Vec<Glyph> = symbol
                            .chars()
                            .map(|c| {
                                let gid = symbol_charmap.map(c).unwrap_or_default();
                                let g = Glyph {
                                    id: gid.to_u32(),
                                    x: x_cursor as f32,
                                    y: y_cursor as f32,
                                };
                                x_cursor += 12.0;
                                g
                            })
                            .collect();

                        scene
                            .draw_glyphs(&symbol_font)
                            .font_size(30.0)
                            .brush(&Color::rgb8(255, 220, 120)) // Gold Labels
                            .draw(vello::peniko::Fill::NonZero, glyphs.into_iter());
                    }

                    // =====================================================
                    // MONTH LABELS
                    // =====================================================
                    let months = [
                        ("Jan", 31),
                        ("Feb", 28),
                        ("Mar", 31),
                        ("Apr", 30),
                        ("May", 31),
                        ("Jun", 30),
                        ("Jul", 31),
                        ("Aug", 31),
                        ("Sep", 30),
                        ("Oct", 31),
                        ("Nov", 30),
                        ("Dec", 31),
                    ];

                    let mut day_cursor = 0.0;
                    let label_radius = outer_radius - 120.0;

                    for (name, len) in months {
                        let angle =
                            NaturalSquaresEngine::day_of_year_to_angle(day_cursor, true) as f64;
                        let p = point_on_circle(center, label_radius, angle);

                        let mut x_cursor = p.x - 10.0;
                        let y_cursor = p.y + 5.0;

                        let glyphs: Vec<Glyph> = name
                            .chars()
                            .map(|c| {
                                let gid = mono_charmap.map(c).unwrap_or_default();
                                let g = Glyph {
                                    id: gid.to_u32(),
                                    x: x_cursor as f32,
                                    y: y_cursor as f32,
                                };
                                x_cursor += 8.0;
                                g
                            })
                            .collect();

                        scene
                            .draw_glyphs(&mono_font)
                            .font_size(14.0)
                            .brush(&Color::rgb8(200, 200, 210))
                            .draw(vello::peniko::Fill::NonZero, glyphs.into_iter());

                        day_cursor += len as f32;
                    }

                    // =====================================================
                    // OCTAVE CROSS & GRID (Linked to Date)
                    // =====================================================
                    // 1. Calculate the progression through the day (fractional days)
                    //    This ensures the octave grid creeps forward smoothly once per second
                    //    matching the exact modern position of the red date marker.
                    let seconds_in_day =
                        (current_hour * 3600.0) + (current_minute * 60.0) + current_second;
                    let day_fraction = seconds_in_day / 86400.0;
                    let precise_today_angle = NaturalSquaresEngine::day_of_year_to_angle(
                        today_day_of_year as f32 + day_fraction as f32,
                        true,
                    ) as f64;

                    // 2. Center "North" (Index 2, which naturally renders at 90°) onto the precise date angle
                    let octave_rotation = precise_today_angle - 90.0;

                    let step = 360.0 / 8.0;
                    let labels = ["E", "NE", "N", "NW", "W", "SW", "S", "SE"];
                    let max_web_layers = 14;

                    // Draw Web Layers
                    for layer in 1..=max_web_layers {
                        let r = layer as f64 * scale;
                        for i in 0..8 {
                            let angle1 = (i as f64 * step) + octave_rotation;
                            let angle2 = ((i + 1) as f64 * step) + octave_rotation;

                            let p1 = point_on_circle(center, r, angle1);
                            let p2 = point_on_circle(center, r, angle2);

                            let web_color = if i % 2 == 0 {
                                Color::rgb8(90, 90, 100)
                            } else {
                                Color::rgb8(93, 99, 100)
                            };

                            scene.stroke(
                                &Stroke::new(0.8),
                                Affine::IDENTITY,
                                web_color,
                                None,
                                &Line::new(p1, p2),
                            );
                        }
                    }

                    // Draw Axis Lines & Direction Labels
                    for i in 0..8 {
                        let angle = (i as f64 * step) + octave_rotation;
                        let end = point_on_circle(center, outer_radius - 160.0, angle);
                        let line = Line::new(center, end);

                        let color = if i % 2 == 0 {
                            Color::rgb8(90, 90, 100)
                        } else {
                            Color::rgb8(93, 99, 100)
                        };

                        scene.stroke(&Stroke::new(1.4), Affine::IDENTITY, color, None, &line);

                        let label_radius = outer_radius - 145.0;
                        let label_pos = point_on_circle(center, label_radius, angle);
                        let label = labels[i];

                        let mut x_cursor = label_pos.x - 8.0;
                        let y_cursor = label_pos.y + 5.0;

                        let glyphs: Vec<Glyph> = label
                            .chars()
                            .map(|c| {
                                let gid = mono_charmap.map(c).unwrap_or_default();
                                let g = Glyph {
                                    id: gid.to_u32(),
                                    x: x_cursor as f32,
                                    y: y_cursor as f32,
                                };
                                x_cursor += 8.0;
                                g
                            })
                            .collect();

                        scene
                            .draw_glyphs(&mono_font)
                            .font_size(11.0)
                            .brush(&Color::rgb8(200, 200, 210))
                            .draw(vello::peniko::Fill::NonZero, glyphs.into_iter());
                    }
                    let mut zodiac_angles = [0.0f64; 13];

                    for i in 0..13 {
                        zodiac_angles[i] = NaturalSquaresEngine::zodiac_to_angle(i as u32) as f64;
                    }

                    let utc_now = chrono::Utc::now();

                    // pick your location (example: Johannesburg)
                    let longitude = 28.0473;
                    let latitude = -26.2041;

                    let is_day = NaturalSquaresEngine::is_daylight(utc_now, longitude, latitude);
                    // =====================================================
                    // SUNLIGHT CONE (FROM dateTick SEGMENT)
                    // =====================================================

                    let r = outer_radius;
                    let today_day_of_year = (today_resolved.ordinal() - 1) as f32;

                    // THIS is your canonical circular position for "today"
                    let date_tick =
                        NaturalSquaresEngine::day_of_year_to_angle(today_day_of_year, true) as f64;
                    let tick = date_tick.rem_euclid(360.0);

                    let mut sun_index = 0usize;

                    // find which zodiac segment contains tick
                    for i in 0..zodiac_angles.len() - 1 {
                        let a0 = zodiac_angles[i].rem_euclid(360.0);
                        let mut b0 = zodiac_angles[i + 1].rem_euclid(360.0);

                        if b0 < a0 {
                            b0 += 360.0;
                        }

                        let mut t = tick;
                        if t < a0 {
                            t += 360.0;
                        }

                        if t >= a0 && t < b0 {
                            sun_index = i;
                            break;
                        }
                    }

                    // Aries (0) → Taurus (1) replaced by selected segment
                    let a = zodiac_angles[sun_index].rem_euclid(360.0);
                    let mut b = zodiac_angles[sun_index + 1].rem_euclid(360.0);

                    // ensure correct sweep direction
                    if b < a {
                        b += 360.0;
                    }

                    let steps = 20;

                    let mut sun_path = BezPath::new();

                    // center
                    sun_path.move_to(center);

                    // build fan
                    for i in 0..=steps {
                        let t = i as f64 / steps as f64;
                        let angle = a + (b - a) * t;

                        let p = point_on_circle(center, r, angle);
                        sun_path.line_to(p);
                    }

                    // close back to center
                    sun_path.close_path();

                    // draw
                    // fix !
                    if is_day {
                        scene.fill(
                            vello::peniko::Fill::NonZero,
                            Affine::IDENTITY,
                            Color::rgba8(255, 210, 80, 35),
                            None,
                            &sun_path,
                        );
                    }

                    // =====================================================
                    // RENDER
                    // =====================================================
                    renderer
                        .render_to_surface(
                            &device_handle.device,
                            &device_handle.queue,
                            &scene,
                            &surface_texture,
                            &render_params,
                        )
                        .unwrap();

                    surface_texture.present();
                }

                _ => {}
            }
        })
        .unwrap();
}

fn main() {
    pollster::block_on(run());
}
