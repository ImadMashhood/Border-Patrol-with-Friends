use super::*;
use bevy_egui::{EguiContexts, egui};
const INK: egui::Color32 = egui::Color32::from_rgb(225, 231, 218);
const GOLD: egui::Color32 = egui::Color32::from_rgb(238, 191, 104);
const MUTED: egui::Color32 = egui::Color32::from_rgb(142, 165, 160);
fn tag(ui: &mut egui::Ui, text: &str) {
    ui.label(egui::RichText::new(text).monospace().size(11.).color(MUTED));
}
fn primary(ui: &mut egui::Ui, label: &str) -> bool {
    ui.add_sized(
        [ui.available_width(), 35.],
        egui::Button::new(
            egui::RichText::new(label)
                .strong()
                .color(egui::Color32::from_rgb(22, 35, 31)),
        )
        .fill(GOLD),
    )
    .clicked()
}
pub fn draw(
    mut contexts: EguiContexts,
    mut g: ResMut<Patrol>,
    net: Res<Network>,
    player: Query<&Transform, With<Player>>,
    vehicles: Query<&traffic::Vehicle>,
    mut traffic_state: ResMut<traffic::Traffic>,
    mut shift: ResMut<shift::Shift>,
) -> Result {
    let ctx = contexts.ctx_mut()?;
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = egui::Color32::from_rgb(17, 30, 29);
    visuals.window_fill = egui::Color32::from_rgb(17, 30, 29);
    visuals.override_text_color = Some(INK);
    visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(25, 42, 39);
    visuals.selection.bg_fill = egui::Color32::from_rgb(55, 91, 72);
    ctx.set_visuals(visuals);
    // A quiet fixed header leaves most of the screen to the checkpoint.
    egui::TopBottomPanel::top("masthead")
        .frame(
            egui::Frame::new()
                .fill(egui::Color32::from_rgb(17, 30, 29))
                .inner_margin(egui::Margin::symmetric(25, 16)),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    tag(ui, "DEPARTMENT OF BORDER AFFAIRS  /  FIELD OPERATIONS");
                    ui.label(egui::RichText::new("SAN IGNACIO").size(24.).strong());
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new(format!(
                            "{:02} / 10",
                            if shift.connected {
                                g.position.min(10)
                            } else {
                                (g.position + 1).min(10)
                            }
                        ))
                        .size(24.)
                        .color(GOLD),
                    );
                    if let Some(world) = &shift.world {
                        let seconds = world.remaining_ms.div_ceil(1000);
                        ui.label(
                            egui::RichText::new(if world.phase == border_net::ShiftPhase::Lobby {
                                "PRE-SHIFT".into()
                            } else {
                                format!("{:02}:{:02}", seconds / 60, seconds % 60)
                            })
                            .size(28.)
                            .color(GOLD),
                        );
                    }
                    tag(ui, &format!("DAY SHIFT  /  SEED {}", g.seed));
                });
            });
        });
    egui::TopBottomPanel::bottom("footer").frame(egui::Frame::new().fill(egui::Color32::from_rgb(17,30,29)).inner_margin(egui::Margin::symmetric(25,12))).show(ctx,|ui| {
        ui.horizontal(|ui| {tag(ui,"W A S D  MOVE   SHIFT  JOG   M  MAP     MOUSE  LOOK     E  INTERACT     TAB  LOOK / TYPE     ESC  RELEASE");ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui|{tag(ui,&format!("{} CORRECT",g.score));
            if g.phase == Phase::Interview {
                tag(ui, if !g.mic_enabled {"MIC OFF"} else if g.mic_failed {"MIC UNAVAILABLE"} else if g.pending || g.speaking || g.speech_pending {"MIC PAUSED"} else {"MIC LISTENING"});
            }});});
    });
    if matches!(
        g.phase,
        Phase::Approach | Phase::Interview | Phase::Departing
    ) {
        let screen = ctx.viewport_rect();
        let center = screen.center();
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Background,
            egui::Id::new("reticle"),
        ));
        if g.looking {
            painter.circle_stroke(
                center,
                if g.aiming { 6. } else { 3. },
                egui::Stroke::new(1.5, if g.aiming { GOLD } else { INK }),
            );
        }
        if g.aiming && !g.overlay {
            egui::Area::new("interact_hint".into())
                .anchor(egui::Align2::CENTER_CENTER, [0., 45.])
                .interactable(false)
                .show(ctx, |ui| {
                    egui::Frame::new()
                        .fill(egui::Color32::from_black_alpha(180))
                        .inner_margin(12.)
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new(
                                    shift
                                        .interaction
                                        .and_then(|(id, point)| {
                                            shift
                                                .world
                                                .as_ref()?
                                                .vehicles
                                                .iter()
                                                .find(|v| v.vehicle_id == id)
                                                .map(|v| format!("[ E ]  {}", point.label(&v.kind)))
                                        })
                                        .unwrap_or_else(|| "[ E ]  Interact".into()),
                                )
                                .color(GOLD)
                                .size(16.),
                            );
                        });
                });
        }
    }
    if !shift.connected
        && g.phase == Phase::Approach
        && !g.overlay
        && let Some(v) = vehicles
            .iter()
            .find(|v| v.lane == 0 && v.ordinal == g.position)
    {
        egui::Area::new("arrival_status".into())
            .anchor(egui::Align2::CENTER_TOP, [0., 95.])
            .interactable(false)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(egui::Color32::from_rgba_unmultiplied(17, 30, 29, 225))
                    .inner_margin(8.)
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new(if v.distance > 0. {
                                format!(
                                    "NEXT TRAVELER APPROACHING  ·  {:.0} m TO STOP",
                                    v.distance.ceil()
                                )
                            } else {
                                "LANE 1  ·  READY FOR INTERVIEW".into()
                            })
                            .monospace()
                            .size(12.)
                            .color(GOLD),
                        );
                    });
            });
    }
    weapon::hud(
        ctx,
        &shift,
        player.single().map(|p| p.translation).unwrap_or_default(),
    );
    shift::draw(ctx, &mut shift);
    if g.map_open {
        site_map(
            ctx,
            player.single().map(|p| p.translation).unwrap_or_default(),
        );
    }
    if !shift.connected
        && g.phase == Phase::Approach
        && !g.looking
        && !g.overlay
        && !std::env::args().any(|a| a.starts_with("--scene-view="))
    {
        egui::Window::new("FIRST WATCH").anchor(egui::Align2::LEFT_BOTTOM,[28.,-75.]).resizable(false).collapsible(false).default_width(340.).show(ctx,|ui| {
            ui.add_space(6.);ui.label(egui::RichText::new("Every story has a crossing.").size(23.).color(GOLD));
            ui.add_space(10.);ui.label("Explore the office, holding yard and river bridge. Press M for the site map. Wait for the next car to stop at Lane 1. Walk to its driver window. Aim at it and press E to begin an interview.");ui.add_space(8.);
            ui.label("Ask about their trip, compare the documents, and request secondary inspection when needed. Decide from evidence.");ui.add_space(12.);
            if primary(ui,"BEGIN PATROL") {g.looking=true;}
            ui.add_space(6.);tag(ui,"10 TRAVELERS · TEXT ALWAYS AVAILABLE");
        });
    }
    if g.overlay && g.phase == Phase::Interview || g.overlay && g.pending && g.case.is_none() {
        egui::SidePanel::right("interview").exact_width(435.).resizable(false).frame(egui::Frame::new().fill(egui::Color32::from_rgba_unmultiplied(17,30,29,248)).inner_margin(22.)).show(ctx,|ui| {
            tag(ui,"INSPECTION RECORD");ui.add_space(7.);
            let (name,occupation,mode)=g.case.as_ref().map(|c|(c.name.clone(),c.occupation.clone(),c.mode.clone())).unwrap_or(("Connecting…".into(),"".into(),"local service".into()));
            ui.label(egui::RichText::new(name).size(29.).strong());tag(ui,&format!("{occupation}  /  {mode}"));ui.add_space(8.);
            ui.horizontal(|ui|{ui.label(egui::RichText::new(g.emotion.clone()).color(GOLD));if ui.small_button("Audio & controls").clicked(){g.settings_open = !g.settings_open;}});
            if g.settings_open {
                tag(ui, if std::env::var("BORDER_SPEECH_URL").is_ok() {"GPU SPEECH: WHISPER TURBO / KOKORO"} else {"LIGHTWEIGHT SPEECH PROFILE"});
                ui.checkbox(&mut g.muted,"Mute dialogue and effects");
                ui.horizontal(|ui|{tag(ui,"CAPTURE DEVICE");ui.text_edit_singleline(&mut g.mic_device);});
                ui.checkbox(&mut g.mic_enabled, "Microphone on — automatic listening");
                ui.add(egui::Slider::new(&mut g.mic_threshold, 0.003..=0.10).text("Noise threshold"));
                if ui.button("Apply settings / retry microphone").clicked() {
                    g.mic = None; g.mic_failed = false; g.mic_error.clear(); g.speech_pending = false;
                }
                ui.label(egui::RichText::new("Speak normally. A short pause sends your question automatically. Listening pauses while the traveler speaks. Use headphones to reduce echo.").size(11.).color(MUTED));
            }
            ui.add_space(8.);ui.separator();
            let transcript_height=(ui.available_height()-360.).clamp(80., 250.);
            egui::ScrollArea::vertical().id_salt("transcript").max_height(transcript_height).stick_to_bottom(true).show(ui,|ui| {
                if let Some(c)=&g.case {tag(ui,"TRAVELER");ui.label(&c.opening_line);ui.add_space(12.);}
                for t in &g.history {tag(ui,"YOU");ui.label(egui::RichText::new(&t.officer).color(GOLD));ui.add_space(6.);tag(ui,"TRAVELER");ui.label(&t.traveler);ui.add_space(14.);}
            });
            ui.add_space(8.);
            if g.pending {ui.horizontal(|ui|{ui.spinner();tag(ui,"THINKING / WAITING FOR SERVICE…");});}
            if g.speaking {tag(ui,"TRAVELER SPEAKING — MICROPHONE PAUSED");}
            else if g.mic.is_some() && !g.pending && !g.speech_pending {tag(ui,if g.mic_active {"HEARING YOU — PAUSE TO SEND"} else {"LISTENING — SPEAK NORMALLY"});}
            else if !g.mic_enabled {tag(ui,"MICROPHONE OFF");}
            if !g.mic_error.is_empty() {ui.colored_label(egui::Color32::from_rgb(245,154,118), &g.mic_error);}
            if g.speech_pending {ui.label("Transcribing locally…");}
            if !g.error.is_empty(){ui.colored_label(egui::Color32::from_rgb(245,154,118),&g.error);}
            let mut send=false;
            ui.add_enabled_ui(!g.pending && !g.speaking && !g.speech_pending && g.case.is_some() && !g.looking,|ui| {
                let response=ui.add(egui::TextEdit::multiline(&mut g.input).desired_rows(2).desired_width(f32::INFINITY).char_limit(500).hint_text("Ask the traveler a question…"));
                if g.request_focus {response.request_focus();g.request_focus=false;}
                send=primary(ui,"SEND QUESTION");
                if response.has_focus() && ui.input(|i|i.key_pressed(egui::Key::Enter) && !i.modifiers.shift) {send=true;}
            });
            if send && !g.input.trim().is_empty() {
                let q=g.input.trim().to_string();g.input.clear();g.error.clear();g.pending=true;
                net.turn(g.generation,g.case.as_ref().unwrap().session_id.clone(),q);
            }
            ui.add_space(5.);
            ui.horizontal_wrapped(|ui|{
                for (label,q) in [("Purpose","What is the purpose of your visit?"),("Cargo","What goods are you carrying?"),("Itinerary","Where will you stay and when will you return?"),("Documents","May I see your documents?")] {
                    if ui.add_enabled(!g.pending,egui::Button::new(label)).clicked(){g.input=q.into();g.looking=false;g.request_focus=true;}
                }
            });
            ui.add_space(7.);ui.separator();ui.add_space(7.);
            ui.add_enabled_ui(!g.pending && !g.speaking && !g.speech_pending && g.case.is_some(),|ui| {
                if ui.button("View documents").clicked(){g.evidence_open=true;}
                if ui.button(if g.secondary{"Secondary inspection completed"}else{"Request secondary inspection"}).clicked(){g.pending=true;g.error.clear();net.inspect(g.generation,g.case.as_ref().unwrap().session_id.clone());}
                ui.add_space(8.);
                ui.horizontal(|ui| {
                    if ui.add_sized([180.,34.],egui::Button::new("ADMIT").fill(egui::Color32::from_rgb(41,91,68))).clicked(){decide(&mut g,&net,border_scenarios::Decision::Admit);}
                    if ui.add_sized([180.,34.],egui::Button::new("DENY").fill(egui::Color32::from_rgb(116,57,43))).clicked(){decide(&mut g,&net,border_scenarios::Decision::Deny);}
                });
            });
            ui.add_space(8.);tag(ui,"FINAL DECISIONS CANNOT BE REVERSED");
        });
    }
    if g.evidence_open && g.phase == Phase::Interview && shift.inspection.is_none() {
        let mut open = true;
        egui::Window::new("EVIDENCE FILE").open(&mut open).default_width(470.).default_pos([45.,150.]).show(ctx,|ui| {
            tag(ui,if g.secondary{"DOCUMENTS & PHYSICAL CARGO"}else{"DOCUMENTS — CARGO NOT YET INSPECTED"});ui.add_space(10.);
            for e in &g.evidence {ui.label(egui::RichText::new(e.id.replace('_'," ").to_uppercase()).color(GOLD).strong());ui.label(&e.description);ui.add_space(14.);}
            if !g.secondary {ui.label("Request secondary inspection to compare physical cargo with the traveler's claims.");}
        });
        g.evidence_open = open;
    }
    if !g.subtitle.is_empty() && !g.overlay && g.phase == Phase::Interview {
        egui::Area::new("subtitle".into())
            .anchor(egui::Align2::CENTER_BOTTOM, [0., -80.])
            .interactable(false)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(egui::Color32::from_black_alpha(195))
                    .inner_margin(16.)
                    .show(ui, |ui| {
                        ui.set_max_width(720.);
                        ui.label(egui::RichText::new(&g.subtitle).size(18.));
                    });
            });
    }
    if g.phase == Phase::Reveal {
        egui::Window::new("CASE DEBRIEF").anchor(egui::Align2::CENTER_CENTER,[0.,0.]).fixed_size([570.,480.]).resizable(false).collapsible(false).show(ctx,|ui| {
            if let Some(r)=&g.reveal {
                tag(ui,"DISPOSITION COMMITTED");ui.add_space(8.);ui.label(egui::RichText::new(if r.correct{"Sound judgment."}else{"Review the evidence."}).size(32.).color(GOLD));
                ui.label(format!("{} — {} — {:?}",r.name,if r.smuggling{"undeclared commercial goods"}else{"honest traveler"},r.decision));ui.add_space(14.);
                egui::ScrollArea::vertical().max_height(300.).show(ui,|ui| {
                    tag(ui,"FACTUAL ACCOUNT");ui.label(&r.truth.purpose);ui.label(&r.truth.itinerary);ui.label(&r.truth.cargo);ui.add_space(12.);
                    tag(ui,"KEY EVIDENCE");if r.clues.is_empty(){ui.label("Documents and physical cargo support the traveler's account. Anxiety, humor and fatigue were not evidence of smuggling.");}
                    for clue in &r.clues {ui.label(clue);ui.add_space(8.);}
                    if !r.missed_clues.is_empty(){tag(ui,"MISSED PHYSICAL CHECK");for clue in &r.missed_clues{ui.label(clue);}}
                });
            }
            ui.add_space(12.);if primary(ui,"CLOSE FILE & CALL NEXT CAR"){if let Some(id)=g.vehicle_id{shift.send(border_net::ClientCommand::ReleaseVehicle{vehicle_id:id});}g.phase=Phase::Departing;g.overlay=false;g.looking=true;}
        });
    }
    if g.phase == Phase::Complete {
        egui::Window::new("SHIFT COMPLETE")
            .anchor(egui::Align2::CENTER_CENTER, [0., 0.])
            .fixed_size([430., 220.])
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                tag(ui, "SAN IGNACIO / END OF WATCH");
                ui.add_space(14.);
                ui.label(
                    egui::RichText::new(format!("{} / 10 correct decisions", g.score))
                        .size(29.)
                        .color(GOLD),
                );
                ui.label(format!(
                    "{} travelers processed. The host can prepare another shift.",
                    g.position
                ));
                ui.add_space(18.);
                if shift.connected {
                    if shift
                        .world
                        .as_ref()
                        .is_some_and(|w| w.host_peer_id == Some(shift.peer_id))
                    {
                        if primary(ui, "PREPARE ANOTHER SHIFT") {
                            shift.send(border_net::ClientCommand::ResetShift);
                        }
                    } else {
                        ui.label("Waiting for the host to prepare the next shift.");
                    }
                } else if primary(ui, "START ANOTHER SHIFT") {
                    traffic_state.restart = true;
                    g.position = 0;
                    g.score = 0;
                    g.reset_case();
                    g.looking = true;
                }
            });
    }
    // Display connection errors even if no session was established.
    if !g.error.is_empty() && g.case.is_none() {
        egui::Window::new("SERVICE STATUS")
            .anchor(egui::Align2::CENTER_CENTER, [0., 0.])
            .collapsible(false)
            .show(ctx, |ui| {
                ui.label(&g.error);
                if ui.button("Retry connection").clicked() {
                    g.error.clear();
                    g.pending = true;
                    if let Some(id) = g.vehicle_id {
                        net.start(g.generation, id.to_string());
                    }
                }
            });
    }
    Ok(())
}
fn decide(g: &mut Patrol, net: &Network, d: border_scenarios::Decision) {
    g.pending = true;
    g.error.clear();
    net.decide(g.generation, g.case.as_ref().unwrap().session_id.clone(), d);
}

/// Project location signs into the world without external font or texture assets.
pub fn landmarks(
    mut contexts: EguiContexts,
    signs: Res<scene::Landmarks>,
    camera: Query<(&Camera, &GlobalTransform), With<Player>>,
    patrol: Res<Patrol>,
    solids: Query<(&Transform, &Solid), Without<Player>>,
    shift: Res<shift::Shift>,
) -> Result {
    if patrol.overlay {
        return Ok(());
    }
    let ctx = contexts.ctx_mut()?;
    let Ok((camera, transform)) = camera.single() else {
        return Ok(());
    };
    let viewport = ctx.viewport_rect();
    for (i, (label, position)) in signs.0.iter().enumerate() {
        let distance = transform.translation().distance(*position);
        if distance > 44. {
            continue;
        }
        let origin = transform.translation();
        let direction = (*position - origin).normalize_or_zero();
        if solids.iter().any(|(t, s)| {
            ray_box(
                origin,
                direction,
                t.translation - s.half,
                t.translation + s.half,
            )
            .is_some_and(|hit| hit < distance - 0.4)
        }) {
            continue;
        }
        if shift.world.as_ref().is_some_and(|w| !w.door_open) {
            let center = Vec3::from_array(border_net::map::DOOR);
            let half = Vec3::new(1.1, 1.5, 0.16);
            if ray_box(origin, direction, center - half, center + half)
                .is_some_and(|hit| hit < distance - 0.4)
            {
                continue;
            }
        }
        let Ok(point) = camera.world_to_viewport(transform, *position) else {
            continue;
        };
        if point.x < 20.
            || point.y < 115.
            || point.x > viewport.width() - 20.
            || point.y > viewport.height() - 75.
        {
            continue;
        }
        egui::Area::new(egui::Id::new(("world_sign", i)))
            .order(egui::Order::Background)
            .interactable(false)
            .pivot(egui::Align2::CENTER_CENTER)
            .fixed_pos(egui::pos2(point.x, point.y))
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(egui::Color32::from_rgba_unmultiplied(20, 38, 32, 220))
                    .inner_margin(5.)
                    .show(ui, |ui| {
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(*label)
                                    .size(if distance < 15. { 13. } else { 11. })
                                    .strong()
                                    .color(GOLD),
                            )
                            .wrap_mode(egui::TextWrapMode::Extend),
                        );
                    });
            });
    }
    Ok(())
}

fn site_map(ctx: &egui::Context, position: Vec3) {
    egui::Window::new("SAN IGNACIO / SITE MAP")
        .anchor(egui::Align2::RIGHT_BOTTOM, [-25., -80.])
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(300., 360.), egui::Sense::hover());
            let painter = ui.painter_at(rect);
            let point = |x: f32, z: f32| {
                egui::pos2(
                    rect.left() + (x + 36.) / 72. * rect.width(),
                    rect.top() + (z + 36.) / 116. * rect.height(),
                )
            };
            let area = |a: (f32, f32), b: (f32, f32)| {
                egui::Rect::from_two_pos(point(a.0, a.1), point(b.0, b.1))
            };
            painter.rect_filled(rect, 2., egui::Color32::from_rgb(171, 137, 91));
            painter.rect_filled(
                area((-30., 36.), (30., 65.)),
                0.,
                egui::Color32::from_rgb(55, 113, 112),
            );
            painter.rect_filled(
                area((-3.5, -34.), (10.5, 79.)),
                0.,
                egui::Color32::from_rgb(62, 69, 64),
            );
            for (name, a, b) in [
                ("OFFICE", (-17., -12.5), (-7., -3.5)),
                ("STORAGE", (-25.5, -12.), (-18.5, -4.)),
                ("HOLDING", (-20., -28.), (-8., -18.)),
                ("SEARCH", (13., -11.), (23., 9.)),
            ] {
                let r = area(a, b);
                painter.rect_filled(r, 1., egui::Color32::from_rgb(33, 63, 51));
                painter.text(
                    r.center(),
                    egui::Align2::CENTER_CENTER,
                    name,
                    egui::FontId::monospace(9.),
                    egui::Color32::WHITE,
                );
            }
            for (a, b) in [((-30., 32.), (-3.7, 32.)), ((10.7, 32.), (30., 32.))] {
                painter.line_segment(
                    [point(a.0, a.1), point(b.0, b.1)],
                    egui::Stroke::new(3., egui::Color32::from_rgb(92, 51, 35)),
                );
            }
            for x in [-3.35, 10.35] {
                painter.line_segment([point(x, 35.), point(x, 66.)], egui::Stroke::new(2., GOLD));
            }
            painter.text(
                point(3.5, 50.),
                egui::Align2::CENTER_CENTER,
                "BRIDGE",
                egui::FontId::monospace(10.),
                GOLD,
            );
            painter.text(
                point(3.5, 0.),
                egui::Align2::CENTER_CENTER,
                "1   2",
                egui::FontId::monospace(12.),
                GOLD,
            );
            painter.text(
                point(-20., 1.),
                egui::Align2::CENTER_CENTER,
                "TEXAS",
                egui::FontId::monospace(11.),
                egui::Color32::WHITE,
            );
            painter.text(
                point(22., 49.),
                egui::Align2::CENTER_CENTER,
                "RIVER",
                egui::FontId::monospace(10.),
                egui::Color32::WHITE,
            );
            painter.circle_filled(point(position.x, position.z), 4., egui::Color32::WHITE);
            ui.label(
                egui::RichText::new("N ↑  •  White dot: you  •  M: close")
                    .size(11.)
                    .color(GOLD),
            );
            ui.label(
                egui::RichText::new(
                    "Checkpoint grounds + bridge are walkable.
Desert and future lanes are scenery.",
                )
                .size(11.)
                .color(MUTED),
            );
        });
}
