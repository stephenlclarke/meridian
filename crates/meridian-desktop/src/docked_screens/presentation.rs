use super::{
    ChartLayout, Context, Control, Frame, Hit, Ink, Key, Label, Line, Marker, ModelPreview,
    ObjectRecord, OfferQuantity, PlanetPreview, RangeRing, Rect, Review, Screen, Stamp, Viewport,
    controller::DockedScreens, money,
};
use meridian_core::station_services::CommodityId;

fn fit(text: &str, width: f64) -> String {
    text.chars()
        .take((width.max(0.0) / 7.5).floor() as usize)
        .collect()
}
pub(super) fn label(f: &mut Frame, text: impl Into<String>, x: f64, y: f64, ink: Ink, clip: Rect) {
    if y >= clip.y && y + 9.0 <= clip.y + clip.height {
        f.labels.push(Label {
            text: fit(&text.into(), (clip.x + clip.width - x).max(0.0)),
            at: [x, y],
            ink,
            clip,
            scale: 1.25,
        });
    }
}
pub(super) fn button(
    f: &mut Frame,
    title: &str,
    r: Rect,
    control: Control,
    enabled: bool,
    reason: Option<&str>,
) {
    f.lines.push(Line {
        from: [r.x, r.y + r.height - 1.0],
        to: [r.x + r.width, r.y + r.height - 1.0],
        ink: if enabled { Ink::Cyan } else { Ink::Blue },
        clip: f.viewport.canvas(),
    });
    label(
        f,
        title,
        r.x + 4.0,
        r.y + 5.0,
        if enabled { Ink::Cyan } else { Ink::Blue },
        r,
    );
    f.hits.push(Hit {
        bounds: r,
        control,
        enabled,
        refusal: reason.map(str::to_owned),
    });
}
fn wrapped(text: &str, columns: usize) -> Vec<String> {
    let width = columns.max(1);
    let mut result = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let chars: Vec<_> = word.chars().collect();
        for part in chars.chunks(width) {
            let chunk: String = part.iter().collect();
            if !current.is_empty() && current.chars().count() + 1 + chunk.chars().count() > width {
                result.push(std::mem::take(&mut current));
            }
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(&chunk);
        }
    }
    if !current.is_empty() {
        result.push(current);
    }
    result
}
pub(super) fn paragraphs(f: &mut Frame, lines: &[String], area: Rect, scroll: usize) {
    let columns = (area.width / 7.5).floor() as usize;
    let rows: Vec<_> = lines.iter().flat_map(|s| wrapped(s, columns)).collect();
    let count = (area.height / 18.0).floor() as usize;
    let start = scroll.min(rows.len().saturating_sub(count.max(1)));
    for (i, text) in rows.iter().skip(start).take(count).enumerate() {
        label(f, text, area.x, area.y + i as f64 * 18.0, Ink::White, area);
    }
}
fn start(selected: usize, count: usize, length: usize) -> usize {
    selected
        .saturating_sub(count.saturating_sub(1))
        .min(length.saturating_sub(count))
}
fn empty_frame(
    viewport: Viewport,
    screen: Option<Screen>,
    stamp: Stamp,
    ui_revision: u64,
) -> Frame {
    Frame {
        stamp,
        ui_revision,
        screen,
        viewport,
        labels: Vec::new(),
        lines: Vec::new(),
        markers: Vec::new(),
        range_rings: Vec::new(),
        navigation_cues: Vec::new(),
        models: Vec::new(),
        planets: Vec::new(),
        hovered: None,
        search_active: false,
        hits: Vec::new(),
        model_bounds: None,
    }
}

pub(crate) fn commander_frame(
    profile: &crate::profile::Profile,
    v: Viewport,
    notice: Option<&str>,
) -> Frame {
    use meridian_core::{FlightMode, navigation::OriginalNavigation};

    let mut f = empty_frame(
        v,
        None,
        Stamp {
            session: 0,
            revision: 0,
        },
        0,
    );
    let canvas = v.canvas();
    label(&mut f, "COMMANDER DETAILS", 16.0, 10.0, Ink::Yellow, canvas);
    f.lines.push(Line {
        from: [16.0, 45.0],
        to: [v.width - 16.0, 45.0],
        ink: Ink::Cyan,
        clip: canvas,
    });
    let controls = if !profile.playing() {
        label(
            &mut f,
            "SAVE SLOT RECOVERY",
            16.0,
            67.0,
            Ink::Orange,
            canvas,
        );
        label(
            &mut f,
            "C Retry loading the existing commander",
            16.0,
            87.0,
            Ink::White,
            canvas,
        );
        "N New commander | Press N twice to confirm replacement"
    } else {
        let world = &profile.world;
        let snapshot = world.snapshot();
        let berth = world.current_dock_context();
        let system = berth
            .map(|b| b.system())
            .or(snapshot.navigation.map(|nav| nav.system));
        let location = system.map_or_else(
            || "Unknown system".into(),
            |id| {
                OriginalNavigation::bundled()
                    .and_then(|catalogue| catalogue.system(id))
                    .map_or_else(
                        |_| format!("Galaxy {} / System {}", id.galaxy, id.system),
                        |entry| entry.name.to_owned(),
                    )
            },
        );
        let status = match (berth, snapshot.mode) {
            (Some(_), _) => "Docked at main station",
            (None, FlightMode::Launching) => "Launching",
            (None, FlightMode::Flight)
                if snapshot.navigation.is_some_and(|nav| nav.travel.is_some()) =>
            {
                "In transit"
            }
            (None, FlightMode::Flight) => "In flight",
            (None, FlightMode::Docked) => "Station berth unavailable",
        };
        let commander = world
            .commander_id()
            .map_or_else(|| "Unavailable".into(), |id| format!("{:016X}", id.seed()));
        let hull = world
            .equipment_snapshot()
            .map_or("Unavailable", |equipment| equipment.definition.hull);
        let ship = world.player_ship_id().map_or_else(
            || hull.to_owned(),
            |id| format!("{hull} / Ship {}", id.ordinal()),
        );
        let credits = world.credits_tenths().map_or_else(
            || "Unavailable".into(),
            |total| format!("{} CR", money(u64::from(total))),
        );
        for (index, row) in [
            format!("Commander ID  {commander}"),
            format!("Ship          {ship}"),
            format!("Location      {location} / {status}"),
            format!("Credits       {credits}"),
        ]
        .into_iter()
        .enumerate()
        {
            label(
                &mut f,
                row,
                16.0,
                67.0 + 20.0 * index as f64,
                Ink::White,
                canvas,
            );
        }
        if berth.is_some() {
            label(
                &mut f,
                "1 Galaxy | 2 System | 3 Showroom | 4 Market | 5 Equipment",
                16.0,
                155.0,
                Ink::Cyan,
                canvas,
            );
            "L Launch | I Mission details | Ctrl+S Save | Ctrl+R Reload"
        } else {
            "Enter Resume flight | Ctrl+S Save | Ctrl+R Reload"
        }
    };
    label(&mut f, controls, 16.0, 175.0, Ink::Cyan, canvas);
    if let Some(notice) = notice {
        let area = Rect::new(16.0, 199.0, v.width - 32.0, v.height - 231.0);
        for (index, row) in wrapped(notice, (area.width / 7.5) as usize)
            .into_iter()
            .enumerate()
        {
            label(
                &mut f,
                row,
                area.x,
                area.y + 16.0 * index as f64,
                Ink::Orange,
                area,
            );
        }
    }
    label(
        &mut f,
        if profile.playing() {
            "H / ? HELP | ESC SAVE / QUIT"
        } else {
            "H / ? HELP | ESC QUIT"
        },
        16.0,
        v.height - 23.0,
        Ink::Cyan,
        canvas,
    );
    f
}

pub(super) fn project(s: &DockedScreens, c: &Context<'_>, v: Viewport) -> Frame {
    let mut f = empty_frame(v, Some(s.screen), c.stamp, s.ui_revision);
    f.hovered = s.hovered;
    f.search_active = s.searching;
    let canvas = v.canvas();
    label(
        &mut f,
        s.screen.title().to_uppercase(),
        16.0,
        10.0,
        Ink::Yellow,
        canvas,
    );
    let mut summary = String::new();
    if let Some(cr) = c.world.credits_tenths() {
        summary = format!("Cash {} CR", money(u64::from(cr)));
    }
    if let Some(service) = c.world.service_snapshot() {
        summary += &format!(
            " | Hold {}/{}t",
            service.used_tonnes, service.hold_capacity_tonnes
        );
    }
    label(&mut f, summary, 16.0, 29.0, Ink::White, canvas);
    let cell = (v.width - 32.0) / Screen::ALL.len() as f64;
    for (index, screen) in Screen::ALL.into_iter().enumerate() {
        button(
            &mut f,
            screen.short_title(),
            Rect::new(16.0 + index as f64 * cell, 49.0, cell - 4.0, 24.0),
            Control::Open(screen),
            true,
            None,
        );
    }
    if c.world.current_dock_context().is_none() {
        label(
            &mut f,
            "Dock at a station to use these screens",
            16.0,
            105.0,
            Ink::Orange,
            canvas,
        );
        f.hits.clear();
        return f;
    }
    if s.route_chooser.is_some() {
        super::route::project(s, c, &mut f);
    } else {
        match s.screen {
            Screen::Galaxy => chart(s, c, &mut f),
            Screen::System => system(s, c, &mut f),
            Screen::Showroom => showroom(s, c, &mut f),
            Screen::Trading => market(s, c, &mut f),
            Screen::Equipment => equipment(s, c, &mut f),
        }
        if matches!(s.screen, Screen::Galaxy | Screen::System)
            && let Some(progress) = c.world.active_route_progress()
        {
            button(
                &mut f,
                &super::route::progress_line(c, &progress),
                Rect::new(16.0, v.height - 47.0, (v.width - 32.0).min(470.0), 18.0),
                Control::OpenRouteProgress,
                true,
                None,
            );
        }
    }
    let footer = Rect::new(16.0, v.height - 28.0, v.width - 110.0, 24.0);
    label(
        &mut f,
        if let Some(panel) = c.equipment.filter(|panel| !panel.feedback.is_empty()) {
            format!("0 COMMANDER | H / ? HELP | {}", panel.feedback)
        } else {
            c.bindings
                .iter()
                .find(|binding| binding.key == Key::Back)
                .map_or_else(
                    || "0 COMMANDER | H / ? HELP".into(),
                    |binding| format!("0 COMMANDER | {} BACK | H / ? HELP", binding.label),
                )
        },
        footer.x,
        footer.y + 5.0,
        Ink::Cyan,
        footer,
    );
    button(
        &mut f,
        "Back",
        Rect::new(v.width - 88.0, v.height - 28.0, 72.0, 24.0),
        Control::Key(Key::Back),
        true,
        None,
    );
    if let Some(review) = c.review {
        modal(c, review, &mut f);
    }
    f
}
fn search(s: &DockedScreens, f: &mut Frame) {
    let r = Rect::new(16.0, 77.0, (f.viewport.width - 144.0).max(80.0), 21.0);
    button(
        f,
        &format!("{}{}", if s.searching { "> " } else { "Search: " }, s.query),
        r,
        Control::Search,
        true,
        None,
    );
    button(
        f,
        "Clear",
        Rect::new(r.x + r.width + 8.0, r.y, 104.0, r.height),
        Control::Key(Key::ClearSearch),
        true,
        None,
    );
}
fn chart(s: &DockedScreens, c: &Context<'_>, f: &mut Frame) {
    let Some(nav) = c.world.snapshot().navigation else {
        return;
    };
    let Some(current) = c.catalogue.world(nav.system) else {
        return;
    };
    let layout = ChartLayout::new(
        f.viewport,
        c.catalogue,
        s.galaxy,
        if s.local {
            Some((current, nav.fuel_units))
        } else {
            None
        },
    );
    let cell = (f.viewport.width - 32.0) / 10.0;
    for galaxy in 0..8u8 {
        button(
            f,
            &format!("G{}", galaxy + 1),
            Rect::new(16.0 + f64::from(galaxy) * cell, 77.0, cell - 4.0, 21.0),
            Control::Galaxy(galaxy),
            !s.local,
            None,
        );
    }
    button(
        f,
        "Here",
        Rect::new(16.0 + 8.0 * cell, 77.0, cell - 4.0, 21.0),
        Control::Key(Key::CurrentLocation),
        true,
        None,
    );
    button(
        f,
        if s.local { "Galaxy" } else { "Local" },
        Rect::new(16.0 + 9.0 * cell, 77.0, cell - 4.0, 21.0),
        Control::Key(Key::LocalChart),
        true,
        None,
    );
    let palette = [
        Ink::Red,
        Ink::RedOrange,
        Ink::Orange,
        Ink::Gold,
        Ink::Yellow,
        Ink::Lime,
        Ink::Leaf,
        Ink::Green,
    ];
    for w in c
        .catalogue
        .worlds
        .iter()
        .filter(|w| w.id.galaxy == s.galaxy)
    {
        let p = layout.point(w);
        if !layout.plot.contains(p) {
            continue;
        }
        let selected = w.id == s.selected_world;
        f.markers.push(Marker {
            system: w.id,
            point: p,
            clip: layout.plot,
            radius: if s.local {
                (f64::from(w.radius_km) * 0.0017).clamp(4.0, 12.0)
            } else {
                (f64::from(w.radius_km) * 0.0004).clamp(1.3, 2.8)
            },
            ink: palette[match s.map_colour {
                1 => usize::from(w.government_index),
                2 => (usize::from(w.technology.saturating_sub(1)) * 7 / 19).min(7),
                _ => usize::from(7 - w.economy_index),
            }],
            current: w.id == nav.system,
            selected,
        });
        if selected {
            label(f, w.name, p[0] + 6.0, p[1] + 6.0, Ink::Yellow, layout.plot);
        }
    }
    if nav.system.galaxy == s.galaxy {
        for (range, ink) in [
            (c.full_reach_ly, Ink::Blue),
            (c.current_reach_ly, Ink::Cyan),
        ] {
            if let Some(radius) = range.and_then(|r| layout.pixels_for_ly(r)) {
                f.range_rings.push(RangeRing {
                    centre: layout.point(current),
                    radius,
                    ink,
                    clip: layout.plot,
                });
            }
        }
    }
    // Nearest-star selection applies anywhere inside the plot, with deterministic ID tie-break.
    button(
        f,
        if s.profit_scan {
            "P: Profit scan ON"
        } else {
            "P: Profit scan OFF"
        },
        Rect::new(
            layout.details.x,
            layout.details.y,
            layout.details.width.min(180.0),
            21.0,
        ),
        Control::Key(Key::ProfitScan),
        true,
        None,
    );
    let mut fields = Vec::new();
    if s.profit_scan {
        fields.extend(profit_fields(c.catalogue, &c.world.profit_details()));
    }
    fields.extend(world_details(s, c));
    let details = Rect::new(
        layout.details.x,
        layout.details.y + 25.0,
        layout.details.width,
        (layout.details.height - 25.0).max(0.0),
    );
    paragraphs(f, &fields, details, s.scroll);
    let search_button = Rect::new(layout.plot.x + 4.0, layout.plot.y + 4.0, 100.0, 21.0);
    button(f, "Find world", search_button, Control::Search, true, None);
    button(
        f,
        ["Economy", "Government", "Technology"][usize::from(s.map_colour)],
        Rect::new(layout.plot.x + 112.0, layout.plot.y + 4.0, 156.0, 21.0),
        Control::Key(Key::MapColour),
        true,
        None,
    );
    let info = Rect::new(
        layout.plot.x + layout.plot.width - 108.0,
        layout.plot.y + 4.0,
        104.0,
        21.0,
    );
    button(
        f,
        "Details",
        info,
        Control::Open(Screen::System),
        true,
        None,
    );
    button(
        f,
        "Plan route",
        Rect::new(info.x - 106.0, info.y, 102.0, 21.0),
        Control::Key(Key::Route),
        c.route_targets.contains(&s.selected_world) && !c.paused,
        Some("This chart destination has no supported route or the game is paused"),
    );
    if s.searching {
        let area = Rect::new(
            layout.plot.x + 8.0,
            layout.plot.y + 32.0,
            (layout.plot.width - 16.0).min(360.0),
            (layout.plot.height - 40.0).max(0.0),
        );
        label(f, format!("> {}", s.query), area.x, area.y, Ink::Cyan, area);
        let rows = c.catalogue.search_worlds(s.galaxy, &s.query);
        let count = f.viewport.rows(area, 22.0);
        let offset = start(s.search_index, count, rows.len());
        if rows.is_empty() {
            label(
                f,
                "No matching world",
                area.x,
                area.y + 22.0,
                Ink::Orange,
                area,
            );
        }
        for (i, w) in rows.iter().skip(offset).take(count).enumerate() {
            let r = Rect::new(area.x, area.y + 22.0 + i as f64 * 20.0, area.width, 20.0);
            button(
                f,
                &format!(
                    "{} {}",
                    if offset + i == s.search_index {
                        ">"
                    } else {
                        " "
                    },
                    w.name
                ),
                r,
                Control::SelectSystem(w.id),
                true,
                None,
            );
        }
    }
}
pub(super) fn profit_fields(
    catalogue: &super::Catalogue,
    results: &[meridian_core::ProfitDetail],
) -> Vec<String> {
    if results.is_empty() {
        return vec!["No profitable destinations in range".into()];
    }
    let mut fields = vec!["Estimated profit (before fuel/risk):".into()];
    for (index, result) in results.iter().enumerate() {
        if let Some(world) = catalogue.world(result.destination.system) {
            fields.push(format!(
                "{}. {}: {} CR",
                index + 1,
                world.name,
                money(result.destination.profit_tenths)
            ));
            fields.push(format!(
                "Owned cargo sale here {} / there {} CR",
                money(result.sale_here_tenths),
                money(result.sale_there_tenths)
            ));
            fields.push(format!(
                "Extra exports est. {} CR",
                money(result.trade_tenths)
            ));
            fields.extend(result.exports.iter().map(export_line));
            if result.exports.is_empty() {
                fields.push("No affordable extra exports".into());
            }
        }
    }
    fields
}

pub(super) fn export_line(row: &meridian_core::navigation::TradeExportEstimate) -> String {
    let name = row
        .commodity
        .definition()
        .map_or("Commodity", |definition| definition.name);
    format!(
        "{} {} {} | ask {} / bid {} CR/unit",
        name,
        row.quantity,
        row.commodity.unit().symbol(),
        money(u64::from(row.ask_tenths)),
        money(u64::from(row.bid_tenths))
    )
}
fn world_details(s: &DockedScreens, c: &Context<'_>) -> Vec<String> {
    let Some(w) = c.catalogue.world(s.selected_world) else {
        return vec!["Select an original world".into()];
    };
    let mut rows = vec![
        format!("G{} / {}", w.id.galaxy + 1, w.name),
        format!("Economy: {}", w.economy),
        format!("Government: {}", w.government),
        format!("Tech level: {}", w.technology),
        format!("Population: {:.1} billion", w.population_billions),
        format!(
            "Gross productivity: {:.0} MCR",
            w.productivity_billion_cr * 1000.0
        ),
        format!("Average radius: {} km", w.radius_km),
        format!("Planet rings: {}", if w.ringed { "Yes" } else { "No" }),
        format!("Surface: {}; climate: {}", w.surface, w.climate),
    ];
    if let Some(nav) = c.world.snapshot().navigation {
        match c.catalogue.distance(nav.system, w.id) {
            Some(d) => {
                rows.push(format!("Distance: {d:.1} light-years"));
                rows.push(
                    c.current_reach_ly
                        .filter(|r| r.is_finite() && *r >= 0.0)
                        .map_or_else(
                            || "Current jump range unavailable".into(),
                            |r| {
                                format!(
                                    "Current reach: {r:.1} ly | Reachable: {}",
                                    if d <= r { "Yes" } else { "No" }
                                )
                            },
                        ),
                );
            }
            None => rows.push("Distance/routes unavailable across galaxies".into()),
        }
    }
    rows.push(if c.route_targets.contains(&w.id) {
        "Route available for review".into()
    } else {
        "Reference only: travel to this system is unavailable".into()
    });
    rows
}
fn system(s: &DockedScreens, c: &Context<'_>, f: &mut Frame) {
    let full_body = f.viewport.body();
    let body = if c.planet_previews && f.viewport.width >= 1000.0 {
        let (facts, planet) = f.viewport.panes();
        if c.catalogue.world(s.selected_world).is_some() {
            f.planets.push(PlanetPreview {
                system: s.selected_world,
                bounds: planet,
            });
        }
        facts
    } else {
        full_body
    };
    let bodies: Vec<_> = c
        .navigation_bodies
        .iter()
        .filter(|b| b.id.system == s.selected_world)
        .collect();
    let body_rows = (bodies.len().min(3) as f64) * 22.0 + if bodies.len() > 3 { 22.0 } else { 0.0 };
    paragraphs(
        f,
        &world_details(s, c),
        Rect::new(
            body.x,
            body.y,
            body.width,
            (body.height - 32.0 - body_rows).max(0.0),
        ),
        s.scroll,
    );
    let body_start = start(s.body_index, 3, bodies.len());
    for (index, target) in bodies.iter().skip(body_start).take(3).enumerate() {
        f.navigation_cues.push((*target).clone());
        let text = target
            .distance_world
            .filter(|d| d.is_finite() && *d >= 0.0)
            .map_or_else(
                || target.name.clone(),
                |d| format!("{}: {d:.0} world units", target.name),
            );
        button(
            f,
            &text,
            Rect::new(
                body.x,
                body.y + body.height - 30.0 - body_rows + index as f64 * 22.0,
                body.width,
                20.0,
            ),
            Control::SelectBody(target.id),
            target.can_target && !c.paused,
            target.refusal.as_deref(),
        );
    }
    if bodies.len() > 3 {
        button(
            f,
            "Previous target",
            Rect::new(
                body.x,
                body.y + body.height - 52.0,
                body.width / 2.0 - 4.0,
                20.0,
            ),
            Control::Key(Key::PreviousBody),
            true,
            None,
        );
        button(
            f,
            "Next target",
            Rect::new(
                body.x + body.width / 2.0,
                body.y + body.height - 52.0,
                body.width / 2.0,
                20.0,
            ),
            Control::Key(Key::NextBody),
            true,
            None,
        );
    }
    button(
        f,
        "Review route",
        Rect::new(body.x, body.y + body.height - 26.0, 160.0, 24.0),
        Control::Key(Key::Route),
        c.route_targets.contains(&s.selected_world) && !c.paused,
        Some("This scene/route is not admitted or the game is paused"),
    );
}
fn showroom(s: &DockedScreens, c: &Context<'_>, f: &mut Frame) {
    search(s, f);
    let (list, side) = f.viewport.panes();
    let details = if s.detail { f.viewport.body() } else { side };
    let rows = c.catalogue.search_ships(&s.query);
    let selected = rows.iter().position(|o| o.id == s.ship);
    let count = f.viewport.rows(list, 0.0);
    let offset = start(selected.unwrap_or(0), count, rows.len());
    for (i, row) in rows
        .iter()
        .skip(offset)
        .take(if s.detail { 0 } else { count })
        .enumerate()
    {
        let r = Rect::new(list.x, list.y + i as f64 * 20.0, list.width, 20.0);
        button(
            f,
            &format!(
                "{} {} {}",
                if row.id == s.ship { ">" } else { " " },
                row.id,
                row.name
            ),
            r,
            Control::SelectShip(row.id.into()),
            true,
            None,
        );
    }
    if rows.is_empty() {
        label(f, "No matching ship", list.x, list.y, Ink::Orange, list);
        return;
    }
    let Some(ship) = selected.and_then(|i| rows.get(i)) else {
        label(
            f,
            "Select a matching ship",
            details.x,
            details.y,
            Ink::Orange,
            details,
        );
        return;
    };
    label(f, ship.name, details.x, details.y, Ink::Yellow, details);
    label(
        f,
        format!(
            "{} ships; {} catalogue objects",
            c.catalogue.ship_count(),
            c.catalogue.object_count()
        ),
        details.x,
        details.y + 18.0,
        Ink::White,
        details,
    );
    showroom_controls(s, f, details);
    let model = Rect::new(
        details.x,
        details.y + 92.0,
        details.width,
        if s.detail {
            0.0
        } else {
            (details.height * 0.32).clamp(0.0, 180.0)
        },
    );
    f.model_bounds = (!s.detail).then_some(model);
    let baseline = s.baseline.as_deref().unwrap_or(c.current_hull);
    showroom_previews(s, c, f, model, ship, baseline);
    let lines = showroom_specifications(s, c, ship, baseline);
    let content = Rect::new(
        details.x,
        model.y + model.height + 8.0,
        details.width,
        (details.y + details.height - model.y - model.height - 64.0).max(0.0),
    );
    paragraphs(f, &lines, content, s.scroll);
    let offer = c.exchanges.iter().find(|o| o.object == ship.id);
    let enabled = offer.is_some_and(|o| o.refusal.is_none()) && !c.paused;
    button(
        f,
        &offer.map_or_else(
            || "Exchange unavailable".into(),
            |o| {
                format!(
                    "Review exchange {} CR / trade-in {} CR",
                    money(o.purchase_tenths),
                    money(o.trade_in_tenths)
                )
            },
        ),
        Rect::new(
            details.x,
            details.y + details.height - 26.0,
            details.width,
            24.0,
        ),
        Control::Key(Key::Exchange),
        enabled,
        offer
            .and_then(|o| o.refusal)
            .or(Some("This ship is not available for exchange")),
    );
}
fn showroom_controls(s: &DockedScreens, f: &mut Frame, details: Rect) {
    let control_width = (details.width / 4.0).max(1.0);
    for (i, (title, key)) in [
        ("Pin", Key::PinBaseline),
        ("Your ship", Key::CurrentShip),
        (if s.compare { "Cmp: on" } else { "Cmp: off" }, Key::Compare),
        (
            if s.rotation_paused { "Rotate" } else { "Pause" },
            Key::Rotate,
        ),
        (["Pitch", "Roll", "Yaw"][usize::from(s.axis)], Key::NextAxis),
        ("Turn -", Key::TurnLeft),
        ("Turn +", Key::TurnRight),
        ("Reset", Key::ResetView),
    ]
    .into_iter()
    .enumerate()
    {
        button(
            f,
            title,
            Rect::new(
                details.x + (i % 4) as f64 * control_width,
                details.y + 40.0 + (i / 4) as f64 * 24.0,
                control_width - 3.0,
                22.0,
            ),
            Control::Key(key),
            true,
            None,
        );
    }
    button(
        f,
        if s.detail {
            "Preview"
        } else {
            "Specifications"
        },
        Rect::new(
            details.x,
            details.y + details.height - 52.0,
            details.width,
            22.0,
        ),
        Control::Key(Key::Details),
        true,
        None,
    );
}

fn showroom_previews(
    s: &DockedScreens,
    c: &Context<'_>,
    f: &mut Frame,
    model: Rect,
    ship: &ObjectRecord<'_>,
    baseline: &str,
) {
    let preview_ids = if s.compare {
        vec![baseline, ship.id]
    } else {
        vec![ship.id]
    };
    let width = model.width / preview_ids.len() as f64;
    for (i, id) in preview_ids
        .into_iter()
        .take(if s.detail { 0 } else { 2 })
        .enumerate()
    {
        let bounds = Rect::new(model.x + i as f64 * width, model.y, width, model.height);
        if let Some(asset) = c.models.iter().find(|m| m.object == id) {
            f.models.push(ModelPreview {
                object: id.into(),
                geometry_key: asset.geometry_key.into(),
                bounds,
                orientation: s.orientation,
            });
        } else {
            label(
                f,
                format!("{id}: preview unavailable"),
                bounds.x,
                bounds.y + 16.0,
                Ink::Blue,
                bounds,
            );
        }
    }
}

fn showroom_specifications(
    s: &DockedScreens,
    c: &Context<'_>,
    ship: &ObjectRecord<'_>,
    baseline: &str,
) -> Vec<String> {
    let mut lines = if ship.id == "SH171" {
        vec![
            "Inspection only. Flight unavailable.".into(),
            "PLUGH fabrication is unavailable.".into(),
            format!(
                "Empty stock dry mass: {} kg",
                meridian_core::medusa::STOCK_EMPTY_DRY.dry_mass_kilograms
            ),
            "Stock figures exclude paid upgrades.".into(),
        ]
    } else {
        vec![
            format!(
                "Authored standard: {} | {}",
                ship.role, ship.purchase_policy
            ),
            format!(
                "Authored flyable hull: {}",
                if ship.player_flyable { "Yes" } else { "No" }
            ),
            "Reference specifications; fitted equipment can change capability.".into(),
        ]
    };
    if s.detail {
        lines.push(format!("Source status: {}", ship.source_status));
    }
    if s.compare {
        lines.push(format!(
            "Left: {} | Right: {}",
            c.catalogue.object(baseline).map_or(baseline, |o| o.name),
            ship.name
        ));
    }
    if let Some(specs) = c.catalogue.authored_specifications(ship.id) {
        lines.extend(specs.iter().cloned());
        return lines;
    }
    for spec in c
        .catalogue
        .specifications
        .iter()
        .filter(|sp| sp.object == ship.id && (s.detail || sp.group == "performance"))
    {
        let comparison = if s.compare {
            c.catalogue
                .specifications
                .iter()
                .find(|a| a.object == baseline && a.group == spec.group && a.key == spec.key)
                .map_or("Unavailable", |a| a.value)
        } else {
            ""
        };
        lines.push(if s.compare {
            format!("{}: {} | {}", spec.key, comparison, spec.value)
        } else {
            format!("{}: {}", specification_label(spec.key), spec.value)
        });
    }
    lines
}

fn category(id: meridian_core::station_services::CommodityId) -> Ink {
    match id.index() {
        3 | 6 | 10 => Ink::Orange,
        0 | 11 => Ink::Green,
        1 | 4 | 5 => Ink::Cyan,
        2 | 7 | 8 | 16 => Ink::Yellow,
        _ => Ink::Blue,
    }
}
fn market(s: &DockedScreens, c: &Context<'_>, f: &mut Frame) {
    let area = f.viewport.body();
    let cell = area.width / 4.0;
    for (i, (title, control)) in [
        ("Buy", Control::TradeMode(false)),
        ("Sell", Control::TradeMode(true)),
        (
            if s.sell {
                "Review sale"
            } else {
                "Review purchase"
            },
            Control::Key(Key::Confirm),
        ),
        ("Refuel (F)", Control::Key(Key::Refuel)),
    ]
    .into_iter()
    .enumerate()
    {
        let refusal = match control {
            Control::Key(Key::Confirm) if c.paused => Some("Resume before trading"),
            Control::Key(Key::Refuel) => super::market_refuel_admission(c.world).err(),
            _ => None,
        };
        button(
            f,
            title,
            Rect::new(area.x + i as f64 * cell, 77.0, cell - 4.0, 21.0),
            control,
            refusal.is_none(),
            refusal,
        );
    }
    for (index, (title, key)) in [
        ("-10", Key::QuantityLessTen),
        ("-1", Key::QuantityLess),
        ("+1", Key::QuantityMore),
        ("+10", Key::QuantityMoreTen),
    ]
    .into_iter()
    .enumerate()
    {
        button(
            f,
            title,
            Rect::new(area.x + index as f64 * 38.0, area.y, 34.0, 20.0),
            Control::Key(key),
            true,
            None,
        );
    }
    label(
        f,
        format!("Qty {} {}", s.quantity, s.commodity.unit().symbol()),
        area.x + 152.0,
        area.y,
        Ink::Yellow,
        area,
    );
    let body = Rect::new(area.x, area.y + 25.0, area.width, area.height - 25.0);
    let offsets = [0.0, 0.38, 0.57, 0.73, 0.87];
    for (i, title) in ["Product", "Price CR", "Stock", "Hold", "Unit"]
        .iter()
        .enumerate()
    {
        label(
            f,
            *title,
            body.x + offsets[i] * body.width,
            body.y,
            Ink::Yellow,
            body,
        );
    }
    let Ok(order) = CommodityId::display_order() else {
        label(
            f,
            "Original commodity content unavailable",
            body.x,
            body.y + 20.0,
            Ink::Orange,
            body,
        );
        return;
    };
    let selected = order.iter().position(|id| *id == s.commodity).unwrap_or(0);
    let count = f.viewport.rows(body, 60.0).max(1);
    let offset = start(selected, count, order.len());
    for (i, id) in order.iter().skip(offset).take(count).enumerate() {
        let y = body.y + 22.0 + i as f64 * 20.0;
        let Ok(definition) = id.definition() else {
            continue;
        };
        let quote = c.world.commodity_quote(*id);
        let price = quote.map(|q| if s.sell { q.bid_tenths } else { q.ask_tenths });
        let held = c.world.commodity_quantity(*id);
        for (column, value) in [
            format!(
                "{}{}",
                if *id == s.commodity { "> " } else { "" },
                definition.name
            ),
            price.map_or_else(|| "Unavailable".into(), |p| money(u64::from(p))),
            quote.map_or_else(|| "-".into(), |q| q.stock.to_string()),
            held.map_or_else(|| "-".into(), |q| q.to_string()),
            definition.unit.symbol().into(),
        ]
        .into_iter()
        .enumerate()
        {
            label(
                f,
                value,
                body.x + offsets[column] * body.width,
                y,
                if column == 0 {
                    category(*id)
                } else {
                    Ink::White
                },
                Rect::new(
                    body.x + offsets[column] * body.width,
                    y,
                    if column < 4 {
                        (offsets[column + 1] - offsets[column]) * body.width - 4.0
                    } else {
                        body.width * 0.13
                    },
                    20.0,
                ),
            );
        }
        f.hits.push(Hit {
            bounds: Rect::new(body.x, y, body.width, 20.0),
            control: Control::SelectCommodity(*id),
            enabled: true,
            refusal: None,
        });
    }
    let reserved = c.world.reserved_commodity_quantity(s.commodity);
    let quote = super::market_refuel_quote(c.world).map_or_else(
        || "Fuel status unavailable".into(),
        |quote| {
            format!(
                "Fuel {:.1}/{:.1} LY | Refuel {} CR",
                f64::from(quote.fuel) / 10.0,
                f64::from(quote.capacity) / 10.0,
                money(u64::from(quote.price))
            )
        },
    );
    if body.height >= 84.0 {
        label(
            f,
            quote,
            body.x,
            body.y + body.height - 40.0,
            Ink::Yellow,
            body,
        );
    }
    let text = reserved.map_or_else(
        || "Cargo custody unavailable".into(),
        |n| {
            if body.height >= 84.0 {
                format!("Reserved: {n}; Mission cargo requires authorisation for sale.")
            } else {
                format!("Reserved: {n} | F Refuel")
            }
        },
    );
    label(
        f,
        text,
        body.x,
        body.y + body.height - 20.0,
        Ink::Orange,
        body,
    );
}
fn equipment(s: &DockedScreens, c: &Context<'_>, f: &mut Frame) {
    let body = f.viewport.body();
    let Some(panel) = c.equipment else {
        paragraphs(
            f,
            &[
                "Equipment service unavailable.".into(),
                "Return to an equipped station to view fitting services.".into(),
            ],
            body,
            0,
        );
        return;
    };
    let slots_per_page = 4usize;
    let selected_slot = panel
        .slots
        .iter()
        .position(|slot| slot.id == panel.selected_slot)
        .unwrap_or(0);
    let slot_start = (selected_slot / slots_per_page) * slots_per_page;
    let cell = body.width / 4.0;
    for (i, slot) in panel.slots.iter().skip(slot_start).take(4).enumerate() {
        button(
            f,
            &format!(
                "{}{}: {}",
                if slot.id == panel.selected_slot {
                    "> "
                } else {
                    ""
                },
                slot.label,
                slot.fitted
            ),
            Rect::new(body.x + i as f64 * cell, 77.0, cell - 4.0, 21.0),
            Control::SelectSlot(slot_start + i),
            slot.available,
            Some("This slot is not present on the current hull"),
        );
    }
    let (list, details) = f.viewport.panes();
    let count = f.viewport.rows(list, 20.0);
    let offset = start(s.equipment_index, count, panel.rows.len());
    for (i, row) in panel.rows.iter().skip(offset).take(count).enumerate() {
        button(
            f,
            &format!(
                "{} {}",
                if offset + i == s.equipment_index {
                    ">"
                } else {
                    " "
                },
                row.name
            ),
            Rect::new(list.x, list.y + i as f64 * 20.0, list.width, 20.0),
            Control::SelectEquipment(offset + i),
            true,
            None,
        );
    }
    let Some(row) = panel.rows.get(s.equipment_index) else {
        label(
            f,
            "No equipment offered at this station",
            details.x,
            details.y,
            Ink::Orange,
            details,
        );
        return;
    };
    let passage_service = panel.selected_slot == "passenger-service";
    let mut lines = vec![
        row.group.clone(),
        row.name.clone(),
        format!(
            "Price: {}",
            row.price_tenths
                .map_or_else(|| "Unavailable".into(), |p| money(p) + " CR")
        ),
        format!("Owned {} | Fitted {}", row.owned, row.fitted),
        row.compatibility.clone(),
        format!(
            "Equipment units {}/{} | Utility mounts {}/{}",
            panel.equipment_units[0],
            panel.equipment_units[1],
            panel.utility_mounts[0],
            panel.utility_mounts[1]
        ),
        match (panel.assigned_crew, panel.required_crew) {
            (Some(assigned), Some(required)) => format!("Crew {assigned}/{required}"),
            _ => "Crew information unavailable".into(),
        },
    ];
    if passage_service {
        // Passage is a separate confirmed action, never a purchase or a free item grant.
        lines.remove(3);
        lines.remove(2);
    }
    if let OfferQuantity::Available(n) = row.quantity {
        lines.push(format!("Available: {n}"));
    }
    lines.extend(row.readout.iter().cloned());
    if let Some(reason) = &row.refusal {
        lines.push(reason.clone());
    }
    if s.options {
        let count = f.viewport.rows(details, 26.0);
        let offset = start(s.option_index, count, row.options.len());
        for (i, option) in row.options.iter().skip(offset).take(count).enumerate() {
            button(
                f,
                &format!(
                    "{} {}: {}",
                    if offset + i == s.option_index {
                        ">"
                    } else {
                        ""
                    },
                    option.label,
                    option.value.to_owned()
                        + &option
                            .price_tenths
                            .map_or_else(String::new, |p| format!(" | {} CR", money(p)))
                ),
                Rect::new(details.x, details.y + i as f64 * 20.0, details.width, 20.0),
                Control::SelectOption(offset + i),
                true,
                None,
            );
        }
    } else {
        paragraphs(
            f,
            &lines,
            Rect::new(
                details.x,
                details.y,
                details.width,
                (details.height - 54.0).max(0.0),
            ),
            s.scroll,
        );
    }
    button(
        f,
        if s.detail {
            "Item focus"
        } else {
            "Detail focus"
        },
        Rect::new(
            details.x,
            details.y + details.height - 50.0,
            details.width,
            20.0,
        ),
        Control::Key(Key::Details),
        true,
        None,
    );
    let action_width = details.width / 4.0;
    for (i, (title, key, enabled)) in [
        (
            if s.options {
                "Review option"
            } else if passage_service {
                "Review"
            } else {
                "Buy"
            },
            Key::Confirm,
            if s.options {
                row.instance.is_some()
            } else {
                row.can_buy
            },
        ),
        ("Fit", Key::Fit, row.can_fit),
        ("Store", Key::Store, row.can_store),
        (
            if s.options { "Back to item" } else { "Options" },
            Key::Options,
            !row.options.is_empty(),
        ),
    ]
    .into_iter()
    .enumerate()
    {
        button(
            f,
            title,
            Rect::new(
                details.x + i as f64 * action_width,
                details.y + details.height - 26.0,
                action_width - 3.0,
                24.0,
            ),
            Control::Key(key),
            enabled && !c.paused && row.refusal.is_none(),
            row.refusal.as_deref().or(Some(
                "This action is unavailable for the selected owned item and slot",
            )),
        );
    }
    button(
        f,
        "Next slot",
        Rect::new(list.x, list.y + list.height - 24.0, list.width, 22.0),
        Control::Key(Key::NextSlot),
        !panel.slots.is_empty(),
        None,
    );
}
fn modal(c: &Context<'_>, review: &Review, f: &mut Frame) {
    // Modal owns all hits. Underlying controls cannot receive a click through the review.
    f.hits.clear();
    f.models.clear();
    f.planets.clear();
    f.markers.clear();
    f.range_rings.clear();
    f.navigation_cues.clear();
    f.lines.clear();
    f.labels.clear();
    f.hovered = None;
    let body = f.viewport.body();
    label(f, &review.title, body.x, body.y, Ink::Yellow, body);
    let mut rows = review.details.clone();
    if let Some(reason) = &review.refusal {
        rows.push(reason.clone());
    }
    paragraphs(
        f,
        &rows,
        Rect::new(
            body.x,
            body.y + 24.0,
            body.width,
            (body.height - 64.0).max(0.0),
        ),
        0,
    );
    button(
        f,
        "Confirm",
        Rect::new(
            body.x,
            body.y + body.height - 28.0,
            body.width / 2.0 - 8.0,
            24.0,
        ),
        Control::ReviewConfirm,
        review.can_confirm && !c.paused,
        review
            .refusal
            .as_deref()
            .or(Some("Confirmation unavailable while paused")),
    );
    button(
        f,
        "Cancel",
        Rect::new(
            body.x + body.width / 2.0,
            body.y + body.height - 28.0,
            body.width / 2.0,
            24.0,
        ),
        Control::ReviewCancel,
        true,
        None,
    );
}

fn specification_label(key: &str) -> &str {
    match key {
        "cargoTonnes" => "Cargo (t)",
        "maxSpeed" => "Maximum speed (project units)",
        "acceleration" => "Acceleration (project units)",
        "energyCapacity" => "Energy capacity",
        "energyRecharge" => "Energy recharge",
        "shieldCapacityPerBank" => "Shield capacity per bank",
        "shieldRechargePerBank" => "Shield recharge per bank",
        "fuelTankLY" => "Fuel tank (ly)",
        "maxJumpLY" => "Maximum jump (ly)",
        "jumpChargeSeconds" => "Jump charge (s)",
        "missileCapacity" => "Missile capacity",
        "laserFacingMask" => "Laser facing mask",
        "cargoExpansionTonnes" => "Cargo expansion (t)",
        "hullPriceCR" => "Reference hull price (CR)",
        "minimumOperatingCrewIncludingCommander" => "Minimum operating crew",
        "intrinsicCrewPlaces" => "Intrinsic crew places",
        "equipmentBudgetUnits" => "Equipment budget",
        "utilityMounts" => "Utility mounts",
        "dimensionsWHLMetres" => "Target dimensions W/H/L (m)",
        _ => key,
    }
}
