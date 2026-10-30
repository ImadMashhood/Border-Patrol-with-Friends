//! Quiet field HUD and the equipment tablet. Displays only public/observed records.
use super::*;
use bevy_egui::egui;
use border_net::{VehicleStage, combat::Side};
use std::collections::{HashMap, VecDeque};
struct Caption {
    speaker: String,
    text: String,
    born: f32,
    until: f32,
    officer: bool,
}
#[derive(Resource, Default)]
pub struct Hud {
    captions: VecDeque<Caption>,
    seen: HashMap<u64, usize>,
    pub cases: HashMap<u64, PublicCase>,
    last_subtitle: (Option<u64>, String),
    pub drew_hud: bool,
    pub drew_hint: bool,
    pub drew_captions: bool,
    pub drew_tablet: bool,
    pub drew_wheel: bool,
}
impl Hud {
    pub fn visible_captions(&self, now: f32) -> bool {
        self.captions.back().is_some_and(|c| alpha(c, now) > 0.95)
    }
    fn caption(&mut self, speaker: String, text: String, officer: bool, now: f32) {
        if text.trim().is_empty()
            || self
                .captions
                .iter()
                .any(|c| c.speaker == speaker && c.text == text && now - c.born < 3.)
        {
            return;
        }
        let duration = (text.chars().count() as f32 * 0.045 + 4.).clamp(6., 14.);
        self.captions.push_back(Caption {
            speaker,
            text,
            born: now,
            until: now + duration,
            officer,
        });
        while self.captions.len() > 4 {
            self.captions.pop_front();
        }
    }
}
pub fn update(mut h: ResMut<Hud>, g: Res<Patrol>, shift: Res<shift::Shift>, time: Res<Time>) {
    let now = time.elapsed_secs();
    if shift
        .world
        .as_ref()
        .is_some_and(|w| w.phase == border_net::ShiftPhase::Lobby)
    {
        h.captions.clear();
        h.seen.clear();
        h.cases.clear();
        h.last_subtitle = (None, String::new());
        return;
    }
    if let (Some(id), Some(case)) = (g.vehicle_id, &g.case) {
        h.cases.insert(id, case.clone());
        let history = shift
            .world
            .as_ref()
            .and_then(|w| w.vehicles.iter().find(|v| v.vehicle_id == id))
            .map(|v| v.history.as_slice())
            .unwrap_or(&g.history);
        let seen = h
            .seen
            .get(&id)
            .copied()
            .unwrap_or(history.len().saturating_sub(1));
        for t in history.iter().skip(seen) {
            let speaker = t.physical.officer_peer.map_or_else(
                || "YOU".into(),
                |p| {
                    if p == shift.peer_id {
                        "YOU".into()
                    } else {
                        format!("OFFICER {p}")
                    }
                },
            );
            h.caption(speaker, t.officer.clone(), true, now);
            h.caption(case.name.clone(), t.traveler.clone(), false, now);
        }
        h.seen.insert(id, history.len());
        if h.last_subtitle != (Some(id), g.subtitle.clone()) {
            h.last_subtitle = (Some(id), g.subtitle.clone());
            if let Some(q) = g.subtitle.strip_prefix("You: ") {
                h.caption("YOU".into(), q.into(), true, now);
            } else if history.is_empty() && g.subtitle == case.opening_line {
                h.caption(case.name.clone(), g.subtitle.clone(), false, now);
            } else if g.subtitle == "Excuse me?" && !g.input.is_empty() {
                h.caption("YOU".into(), g.input.clone(), true, now);
                h.caption(case.name.clone(), g.subtitle.clone(), false, now);
            }
        }
    }
    if g.speaking
        && let Some(c) = h.captions.back_mut()
        && !c.officer
    {
        c.until = c.until.max(now + 2.);
    }
    h.captions.retain(|c| c.until > now);
}
fn alpha(c: &Caption, now: f32) -> f32 {
    ((now - c.born) / 0.2)
        .min((c.until - now) / 1.2)
        .clamp(0., 1.)
}
fn frame() -> egui::Frame {
    egui::Frame::new()
        .fill(egui::Color32::from_rgba_unmultiplied(14, 25, 23, 222))
        .corner_radius(7.)
        .inner_margin(12.)
}
pub fn hint(target: interactions::Target, g: &Patrol, shift: &shift::Shift) -> String {
    use interactions::Action;
    let v = shift
        .world
        .as_ref()
        .and_then(|w| w.vehicles.iter().find(|v| v.vehicle_id == target.vehicle));
    match target.action {
        Action::Talk => {
            if g.vehicle_id == Some(target.vehicle) && g.case.is_some() {
                "[E] Finish conversation".into()
            } else {
                format!(
                    "[E] Talk to {}",
                    v.map_or("traveler", |v| v.traveler_name.as_str())
                )
            }
        }
        Action::Door(side) => {
            let open =
                v.is_some_and(|v| v.physics.door_open[if side == Side::Driver { 0 } else { 1 }]);
            format!(
                "[E] {} {} door",
                if open { "Close" } else { "Open" },
                if side == Side::Driver {
                    "driver"
                } else {
                    "passenger"
                }
            )
        }
        Action::Panel(point) => format!(
            "[E] {} {}",
            if v.is_some_and(|v| v.open_panels.contains(&point)) {
                "Close"
            } else {
                "Open / inspect"
            },
            v.map_or("panel", |v| point.label(&v.kind))
                .split('·')
                .next()
                .unwrap_or("panel")
                .trim()
                .to_lowercase()
        ),
        Action::Compartment(location) => format!("[E] Search {}", location.label().to_lowercase()),
        Action::Body(_) => "[HOLD E] Grab body · release to drop".into(),
        Action::Porter(_) => "[E] Ask porter to clear vehicle".into(),
        Action::Ready => "[HOLD E · 2s] Start shift".into(),
    }
}
#[allow(clippy::too_many_arguments)]
pub fn draw(
    ctx: &egui::Context,
    h: &mut Hud,
    e: &mut equipment::Equipment,
    g: &mut Patrol,
    net: &Network,
    shift: &mut shift::Shift,
    focus: &interactions::Focus,
    now: f32,
    camera: Option<(&Camera, &GlobalTransform)>,
) {
    let mut visuals = egui::Visuals::dark();
    visuals.override_text_color = Some(egui::Color32::from_rgb(228, 235, 229));
    visuals.window_fill = egui::Color32::from_rgb(17, 29, 27);
    visuals.selection.bg_fill = egui::Color32::from_rgb(51, 83, 70);
    ctx.set_visuals(visuals);
    if !e.wheel && !g.overlay && !g.map_open && g.looking {
        ctx.layer_painter(egui::LayerId::background())
            .circle_stroke(
                ctx.viewport_rect().center(),
                2.5,
                egui::Stroke::new(1., egui::Color32::WHITE),
            );
    }
    if let Some(p) = shift.me() {
        h.drew_hud = true;
        egui::Area::new("field-loadout".into())
            .anchor(egui::Align2::LEFT_BOTTOM, [22., -20.])
            .interactable(false)
            .show(ctx, |ui| {
                frame().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(if p.weapon.drawn {
                                "PISTOL"
                            } else {
                                e.selected.name()
                            })
                            .strong()
                            .size(15.),
                        );
                        ui.separator();
                        ui.label(
                            egui::RichText::new(format!(
                                "{:02} / {:02}",
                                p.weapon.magazine, p.weapon.reserve
                            ))
                            .monospace()
                            .size(20.)
                            .color(ui::GOLD),
                        );
                        ui.small(if p.health == 0 {
                            "INCAPACITATED"
                        } else if p.weapon.reload_until.is_some() {
                            "RELOADING"
                        } else if p.weapon.drawn {
                            "9 mm"
                        } else {
                            "PISTOL HOLSTERED"
                        });
                    });
                    ui.label(
                        egui::RichText::new(if e.selected == equipment::Item::Tablet {
                            "HOLD TAB  Equipment     E / I  Open tablet"
                        } else {
                            "HOLD TAB  Equipment     B  Draw / holster     R  Reload"
                        })
                        .size(11.)
                        .color(egui::Color32::from_gray(175)),
                    );
                });
            });
    }
    if !g.overlay && !e.wheel && !g.map_open {
        if e.selected == equipment::Item::Tablet {
            egui::Area::new("tablet-use-hint".into())
                .anchor(egui::Align2::CENTER_BOTTOM, [0., -90.])
                .interactable(false)
                .show(ctx, |ui| {
                    ui.label("[E] Open tablet");
                });
        } else if let (Some(target), Some(point), Some((camera, transform))) =
            (focus.target, focus.point, camera)
            && let Ok(pos) = camera.world_to_viewport(transform, point)
        {
            h.drew_hint = true;
            egui::Area::new("focused-object-action".into())
                .pivot(egui::Align2::CENTER_TOP)
                .fixed_pos(egui::pos2(pos.x, pos.y + 24.))
                .interactable(false)
                .show(ctx, |ui| {
                    egui::Frame::new()
                        .fill(egui::Color32::from_black_alpha(185))
                        .corner_radius(4.)
                        .inner_margin(7.)
                        .show(ui, |ui| {
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(hint(target, g, shift))
                                        .size(13.)
                                        .color(egui::Color32::WHITE),
                                )
                                .wrap_mode(egui::TextWrapMode::Extend),
                            );
                        });
                });
        }
        if shift.world.as_ref().is_some_and(|w| {
            w.vehicles
                .iter()
                .any(|v| v.physics.npc.body_holder == Some(shift.peer_id))
        }) {
            egui::Area::new("body-grip-hint".into())
                .anchor(egui::Align2::CENTER_BOTTOM, [0., -100.])
                .interactable(false)
                .show(ctx, |ui| {
                    ui.small("HOLD E · Carrying body     Release E to drop");
                });
        }
        if !h.captions.is_empty() {
            h.drew_captions = true;
            egui::Area::new("field-conversation".into())
                .anchor(egui::Align2::RIGHT_TOP, [-24., 24.])
                .interactable(false)
                .show(ctx, |ui| {
                    ui.set_width((ctx.viewport_rect().width() * 0.29).clamp(220., 390.));
                    for c in &h.captions {
                        let opacity = alpha(c, now);
                        ui.scope(|ui| {
                            ui.set_opacity(opacity);
                            egui::Frame::new()
                                .fill(egui::Color32::from_black_alpha(155))
                                .corner_radius(5.)
                                .inner_margin(9.)
                                .show(ui, |ui| {
                                    ui.label(
                                        egui::RichText::new(&c.speaker).size(10.).strong().color(
                                            if c.officer {
                                                ui::GOLD
                                            } else {
                                                egui::Color32::from_gray(190)
                                            },
                                        ),
                                    );
                                    ui.label(egui::RichText::new(&c.text).size(13.));
                                });
                        });
                        ui.add_space(5.);
                    }
                });
        }
    }
    if g.map_open {
        ui::site_map(
            ctx,
            shift
                .me()
                .map(|p| Vec3::from_array(p.position))
                .unwrap_or_default(),
        );
    }
    if e.wheel {
        h.drew_wheel = true;
        equipment::wheel(ctx, e);
    } else if g.overlay {
        h.drew_tablet = true;
        tablet(ctx, h, e, g, net, shift, now);
    }
}
fn can_decide(shift: &shift::Shift, v: &border_net::VehicleView) -> bool {
    shift.me().is_some_and(|p| {
        p.health > 0 && shift.permits(v.lane_id) && {
            let window = border_net::vehicle::spec(&v.kind).world_point(
                v.position,
                v.yaw,
                border_net::vehicle::VehiclePoint::Driver,
            );
            (p.position[0] - window[0]).hypot(p.position[2] - window[2])
                <= border_net::vehicle::INTERACTION_RANGE
        }
    }) && v.physics.npc.health > 0
        && v.stage == VehicleStage::Stopped
        && !matches!(
            v.physics.npc.mode,
            border_net::combat::NpcMode::Entering | border_net::combat::NpcMode::Exiting
        )
}
#[allow(clippy::too_many_arguments)]
fn tablet(
    ctx: &egui::Context,
    h: &Hud,
    e: &mut equipment::Equipment,
    g: &mut Patrol,
    net: &Network,
    shift: &mut shift::Shift,
    now: f32,
) {
    let mut clear_error = false;
    let Some(world) = &shift.world else {
        return;
    };
    if e.record.is_none()
        || !world
            .vehicles
            .iter()
            .any(|v| Some(v.vehicle_id) == e.record)
    {
        e.record = g.vehicle_id.or_else(|| {
            world
                .vehicles
                .iter()
                .find(|v| shift.permits(v.lane_id) && v.stage == VehicleStage::Stopped)
                .map(|v| v.vehicle_id)
        });
    }
    if let Some((id, deadline)) = e.awaiting
        && (now > deadline
            || !shift.error.is_empty()
            || world
                .vehicles
                .iter()
                .any(|v| v.vehicle_id == id && v.outcome.is_some()))
    {
        e.awaiting = None;
    }
    let screen = ctx.viewport_rect();
    let size = egui::vec2(
        (screen.width() - 80.).clamp(540., 1050.),
        (screen.height() - 140.).clamp(380., 740.),
    );
    let mut open = true;
    egui::Window::new("SAN IGNACIO  /  FIELD TABLET").id("field-tablet".into()).anchor(egui::Align2::CENTER_CENTER,[0.,-10.]).fixed_size(size).collapsible(false).resizable(false).open(&mut open)
        .frame(egui::Frame::window(&ctx.style()).fill(egui::Color32::from_rgb(16,28,26)).stroke(egui::Stroke::new(7.,egui::Color32::from_rgb(38,44,42))).corner_radius(14.).inner_margin(22.))
        .show(ctx,|ui|{
            ui.horizontal(|ui|{
                ui.label(egui::RichText::new("BORDER AFFAIRS · INSPECTION RECORDS").monospace().size(12.).color(ui::GOLD));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui|{ui.small(format!("FUNDS ${}  ·  {:02}:{:02}",world.money,world.remaining_ms/60000,world.remaining_ms/1000%60));});
            });
            ui.add_space(10.);
            ui.horizontal(|ui|{
                egui::ComboBox::from_id_salt("tablet-case").width(300.).selected_text(e.record.and_then(|id|world.vehicles.iter().find(|v|v.vehicle_id==id)).map_or("Select a traveler".into(),|v|format!("Lane {} · #{} {}",v.lane_id,v.vehicle_id,v.traveler_name))).show_ui(ui,|ui|{
                    for v in &world.vehicles {ui.selectable_value(&mut e.record,Some(v.vehicle_id),format!("Lane {} · #{} {} · {:?}",v.lane_id,v.vehicle_id,v.traveler_name,v.stage));}
                });
                for (i,label) in ["Details & documents","Conversation","Shift & audio"].into_iter().enumerate() {ui.selectable_value(&mut e.page,i,label);}
            });
            ui.separator();
            let selected=world.vehicles.iter().find(|v|Some(v.vehicle_id)==e.record);
            let footer=if selected.is_some(){145.}else{35.};
            egui::ScrollArea::vertical().id_salt("tablet-content").max_height((ui.available_height()-footer).max(100.)).auto_shrink([false,false]).show(ui,|ui|{
                if e.page==2 {
                    ui.heading("Shift & equipment");
                    ui.label(format!("Officer {} · {:?} · Seed {}",shift.peer_id,world.phase,world.seed));
                    for lane in &world.lanes {ui.horizontal(|ui|{ui.label(format!("Lane {} · Officer {} · {} waiting",lane.lane_id,lane.owner.map_or("unassigned".into(),|id|id.to_string()),lane.queue.len()));if ui.button("Claim lane").clicked(){shift.send(border_net::ClientCommand::ClaimLane{lane_id:lane.lane_id});}});}
                    ui.separator();ui.heading("Audio");
                    ui.checkbox(&mut g.muted,"Mute dialogue and effects");ui.checkbox(&mut g.mic_enabled,"Automatic microphone");
                    ui.horizontal(|ui|{ui.label("Capture device");ui.text_edit_singleline(&mut g.mic_device);});
                    ui.add(egui::Slider::new(&mut g.mic_threshold,0.003..=0.10).text("Noise threshold"));
                    if ui.button("Apply / retry microphone").clicked(){g.mic=None;g.mic_failed=false;g.mic_error.clear();g.speech_pending=false;}
                    ui.small("1.5 seconds of silence sends speech; maximum 20 seconds. Listening pauses while the traveler speaks.");
                    if !g.mic_error.is_empty(){ui.label(&g.mic_error);}
                    if world.phase==border_net::ShiftPhase::Grace && world.host_peer_id==Some(shift.peer_id) && ui.button("Prepare another shift").clicked(){shift.send(border_net::ClientCommand::ResetShift);}
                    for notice in world.notices.iter().rev().take(8){ui.small(&notice.message);}
                } else if let Some(v)=selected {
                    ui.heading(&v.traveler_name);
                    ui.label(format!("{} · Lane {} · {:?}",v.kind.replace('_'," "),v.lane_id,v.stage));
                    if let Some(case)=h.cases.get(&v.vehicle_id){ui.small(&case.occupation);}
                    ui.add_space(10.);
                    if e.page==0 {
                        ui.label(egui::RichText::new("RECORDED DECLARATION").strong().color(ui::GOLD));
                        ui.label(&v.opening_line);
                        if let Some(case)=h.cases.get(&v.vehicle_id){for claim in &case.declared_claims {ui.label(&claim.text);}}
                        ui.add_space(12.);ui.separator();ui.heading("Documents & search findings");
                        if v.observed_evidence.is_empty(){ui.label("No documents or physical findings recorded. Speak to the driver and inspect the vehicle.");}
                        for finding in &v.observed_evidence {
                            ui.add_space(9.);ui.label(egui::RichText::new(format!("{} · {}",finding.location.label(),finding.id.replace('_'," "))).strong());ui.label(&finding.description);
                            for id in &finding.contradicts {
                                let claim=h.cases.get(&v.vehicle_id).and_then(|c|c.declared_claims.iter().find(|c|c.id==*id)).map_or(id.as_str(),|c|c.text.as_str());
                                ui.colored_label(ui::GOLD,format!("Conflicts with declaration: {claim}"));
                            }
                        }
                        ui.add_space(12.);ui.small(format!("Searched: {}",if v.searched_compartments.is_empty(){"none".into()}else{v.searched_compartments.iter().map(|l|l.label()).collect::<Vec<_>>().join(", ")}));
                    } else {
                        ui.label(egui::RichText::new("TRAVELER").small().color(ui::GOLD));ui.label(&v.opening_line);
                        for turn in &v.history {ui.add_space(10.);ui.label(egui::RichText::new(turn.physical.officer_peer.map_or("OFFICER".into(),|id|format!("OFFICER {id}"))).small().color(ui::GOLD));ui.label(&turn.officer);ui.add_space(4.);ui.small(&v.traveler_name);ui.label(&turn.traveler);}
                        if g.vehicle_id==Some(v.vehicle_id) && g.phase==Phase::Interview {
                            ui.separator();ui.add_enabled_ui(!g.pending && !g.speaking,|ui|{
                                ui.add(egui::TextEdit::multiline(&mut g.input).desired_rows(2).desired_width(f32::INFINITY).hint_text("Type a question…"));
                                if ui.button("Send question").clicked() && !g.input.trim().is_empty(){let q=g.input.trim().to_owned();g.input.clear();g.subtitle=format!("You: {q}");g.pending=true;net.turn(g.generation,v.vehicle_id.to_string(),q);}
                            });
                        }
                    }
                } else {ui.heading("No traveler selected");ui.label("Start the shift, then select a vehicle record above.");}
            });
            ui.separator();
            if let Some(v)=selected {
                if let Some(outcome)=&v.outcome {
                    ui.colored_label(ui::GOLD,format!("DECISION RECORDED · {:?}",outcome.decision));
                    ui.small(if outcome.correct {"Disposition confirmed."}else{"See the recorded evidence before the next decision."});
                    ui.add_enabled_ui(shift.permits(v.lane_id) && v.stage==VehicleStage::Resolved,|ui|{if ui.button("Release vehicle / call next").clicked(){shift.send(border_net::ClientCommand::ReleaseVehicle{vehicle_id:v.vehicle_id});g.overlay=false;g.looking=true;}});
                } else {
                    let ready=can_decide(shift,v) && e.awaiting.is_none();
                    ui.horizontal(|ui|{
                        for (label,decision,color) in [("APPROVE ENTRY",border_net::Disposition::Admit,egui::Color32::from_rgb(40,92,66)),("DISAPPROVE ENTRY",border_net::Disposition::Deny,egui::Color32::from_rgb(120,56,43)),("DETAIN",border_net::Disposition::Detain,egui::Color32::from_rgb(81,75,52))] {
                            let supported=decision!=border_net::Disposition::Detain || v.observed_evidence.iter().any(|x|!x.contradicts.is_empty());
                            if ui.add_enabled(ready && supported,egui::Button::new(label).fill(color).min_size(egui::vec2(150.,32.))).clicked(){clear_error=true;submit_decision(e,shift,v.vehicle_id,decision,now);}
                        }
                    });
                    ui.small(if e.awaiting.is_some(){"Submitting decision to host…"}else if ready {"Decisions are final. Detention requires recorded contradictory evidence."}else{"Return to this living driver's window in your assigned / assisted lane to decide."});
                    if ui.add_enabled(ready,egui::Button::new(if v.secondary_requested {"Secondary inspection requested"}else{"Request secondary inspection"})).clicked(){shift.send(border_net::ClientCommand::RequestSecondary{vehicle_id:v.vehicle_id});}
                }
            }
            if !shift.error.is_empty(){ui.colored_label(egui::Color32::from_rgb(240,157,124),&shift.error);}
            ui.small("Esc / I closes tablet · Hold Tab to change equipment");
        });
    if clear_error {
        shift.error.clear();
    }
    if !open {
        g.overlay = false;
        g.looking = true;
    }
}

/// Shared tablet button/controller path; the host repeats every authority check.
pub fn submit_decision(
    e: &mut equipment::Equipment,
    shift: &shift::Shift,
    id: u64,
    disposition: border_net::Disposition,
    now: f32,
) -> bool {
    let Some(v) = shift
        .world
        .as_ref()
        .and_then(|w| w.vehicles.iter().find(|v| v.vehicle_id == id))
    else {
        return false;
    };
    if !can_decide(shift, v)
        || e.awaiting.is_some()
        || (disposition == border_net::Disposition::Detain
            && !v
                .observed_evidence
                .iter()
                .any(|x| !x.contradicts.is_empty()))
    {
        return false;
    }
    e.awaiting = Some((id, now + 5.));
    shift.send(border_net::ClientCommand::Decide {
        vehicle_id: id,
        disposition,
    });
    true
}
