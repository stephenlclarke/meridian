//! Read-only projections for the selected course and its geographic legs.
use super::{
    Context, Control, Frame, Ink, Line, Outcome, Rect, Request,
    controller::DockedScreens,
    money,
    presentation::{button, export_line, label, paragraphs},
};
use meridian_core::navigation::{
    RouteAlternative, RoutePlan, RouteProgress, RouteStatus, SystemId,
};

#[derive(Debug)]
pub(super) struct RouteChooser {
    pub(super) candidates: Vec<RouteAlternative>,
    pub(super) selected: usize,
    pub(super) trade_detail: bool,
    pub(super) detail_scroll: usize,
}
impl RouteChooser {
    pub(super) fn choose(&mut self, index: usize) -> Outcome {
        if index >= self.candidates.len() {
            return Outcome::Ignored;
        }
        self.selected = index;
        self.detail_scroll = 0;
        Outcome::Updated
    }

    pub(super) fn confirm(&self) -> Outcome {
        self.candidates
            .get(self.selected)
            .map_or(Outcome::Ignored, |route| {
                Outcome::Request(Request::SetNavigationRoute(route.plan.clone()))
            })
    }

    pub(super) fn toggle_trade_detail(&mut self) -> Outcome {
        if self.candidates.get(self.selected).is_none() {
            return Outcome::Ignored;
        }
        self.trade_detail = !self.trade_detail;
        self.detail_scroll = 0;
        Outcome::Updated
    }

    pub(super) fn scroll_details(&mut self, amount: i32, context: &Context<'_>) -> Outcome {
        let Some(route) = self.candidates.get(self.selected) else {
            return Outcome::Ignored;
        };
        self.detail_scroll = super::moved(
            self.detail_scroll,
            amount,
            trade_detail_rows(context, route).len(),
        );
        Outcome::Updated
    }
}

pub(super) fn progress_line(context: &Context<'_>, route: &RouteProgress) -> String {
    let destination = world_name(context, route.plan.destination());
    let legs = route.plan.systems().len() - 1;
    let stop = usize::from(route.completed_legs);
    let next = route.plan.systems().get(stop + 1).copied();
    let status = match route.status {
        RouteStatus::Ready => next.map_or_else(String::new, |id| {
            format!("Next {}", world_name(context, id))
        }),
        RouteStatus::RefuelRequired => format!(
            "Refuel to {:.1} for {}",
            f64::from(route.next_fuel_units.unwrap_or_default()) / 10.0,
            next.map_or("next stop", |id| world_name(context, id))
        ),
        RouteStatus::AwaitingDock => "Dock to confirm arrival".into(),
        RouteStatus::ComputerUnavailable => "Fit a suitable navigation computer".into(),
        RouteStatus::Complete => "Destination reached".into(),
    };
    format!("{destination} | {stop}/{legs} legs | {status}")
}

pub(super) fn project(s: &DockedScreens, context: &Context<'_>, frame: &mut Frame) {
    let Some(chooser) = &s.route_chooser else {
        return;
    };
    let body = frame.viewport.body();
    let (list, plot) = frame.viewport.panes();
    let active = context.world.active_route_progress();
    let selected = chooser.candidates.get(chooser.selected);
    if chooser.trade_detail
        && let Some(route) = selected
    {
        project_trade_detail(frame, context, chooser, route);
        return;
    }
    let heading = selected.map_or_else(
        || "Active route".into(),
        |route| {
            format!(
                "Routes to {} | {}",
                world_name(context, route.plan.destination()),
                route.plan.grade().label()
            )
        },
    );
    label(frame, heading, body.x, body.y, Ink::Yellow, body);
    for (index, route) in chooser.candidates.iter().enumerate() {
        let preference = route
            .preferences
            .first()
            .map_or("Route", |preference| preference.label());
        let prefix = if index == chooser.selected { ">" } else { " " };
        button(
            frame,
            &format!("{prefix} {preference}"),
            Rect::new(
                list.x,
                list.y + 16.0 + index as f64 * 20.0,
                list.width,
                20.0,
            ),
            Control::RouteCandidate(index),
            true,
            None,
        );
    }
    if let Some(route) = selected {
        let facts = [
            format!(
                "{:.2} LY | {:.1} fuel",
                route.distance_ly,
                f64::from(route.fuel_units) / 10.0
            ),
            format!(
                "Risk {:.2} | Trade est. {} CR",
                route.risk,
                money(route.trade_tenths)
            ),
            format!("Best cargo sale est. {} CR", money(route.cargo_sale_tenths)),
            route
                .preferences
                .iter()
                .map(|value| value.label())
                .collect::<Vec<_>>()
                .join(" / "),
            if route.plan.systems().len() > 2 {
                "Trade legs priced separately".into()
            } else {
                "Based on current quotes".into()
            },
            if route.plan.systems().len() > 2 {
                "Refuel at intermediate stops as needed".into()
            } else {
                String::new()
            },
        ];
        for (index, fact) in facts.into_iter().enumerate() {
            let content = Rect::new(list.x, list.y, list.width, (list.height - 43.0).max(0.0));
            label(
                frame,
                fact,
                list.x,
                list.y + 62.0 + index as f64 * 14.0,
                Ink::White,
                content,
            );
        }
    }
    if let Some(plan) = selected
        .map(|route| &route.plan)
        .or_else(|| active.as_ref().map(|route| &route.plan))
    {
        plot_course(
            frame,
            context,
            Rect::new(plot.x, plot.y + 15.0, plot.width, plot.height - 51.0),
            plan,
        );
    }
    if let Some(route) = &active {
        let y = if selected.is_none() {
            body.y + 16.0
        } else {
            body.y + body.height - 42.0
        };
        label(
            frame,
            progress_line(context, route),
            body.x,
            y,
            Ink::Cyan,
            body,
        );
    }
    let actions = Rect::new(body.x, body.y + body.height - 22.0, body.width, 22.0);
    if selected.is_some() {
        button(
            frame,
            "Set route",
            Rect::new(actions.x, actions.y, 90.0, actions.height),
            Control::Key(super::Key::Confirm),
            !context.paused,
            Some("Resume before setting a route"),
        );
        if actions.width >= 400.0 || active.is_none() {
            button(
                frame,
                "D Trade details",
                Rect::new(
                    actions.x + actions.width - 120.0,
                    actions.y,
                    120.0,
                    actions.height,
                ),
                Control::Key(super::Key::Details),
                true,
                None,
            );
        }
    }
    if let Some(route) = active {
        button(
            frame,
            "Clear route",
            Rect::new(actions.x + 94.0, actions.y, 94.0, actions.height),
            Control::ClearNavigationRoute,
            !context.paused,
            Some("Resume before clearing a route"),
        );
        if route.status == RouteStatus::RefuelRequired {
            button(
                frame,
                "Refuel",
                Rect::new(actions.x + 192.0, actions.y, 78.0, actions.height),
                Control::RefuelNavigationRoute,
                !context.paused,
                Some("Resume before refuelling"),
            );
        }
    }
}

fn trade_detail_rows(context: &Context<'_>, route: &RouteAlternative) -> Vec<String> {
    let mut rows = vec![
        "Current quotes when planned. Reopen to update.".into(),
        "Each leg is estimated independently; no purchase or sale.".into(),
    ];
    for leg in &route.trade_legs {
        rows.push(format!(
            "{} > {} | exports est. {} CR",
            world_name(context, leg.from),
            world_name(context, leg.to),
            money(leg.profit_tenths)
        ));
        if leg.exports.is_empty() {
            rows.push("No affordable extra exports".into());
        }
        rows.extend(leg.exports.iter().map(export_line));
    }
    rows
}

fn project_trade_detail(
    frame: &mut Frame,
    context: &Context<'_>,
    chooser: &RouteChooser,
    route: &RouteAlternative,
) {
    let body = frame.viewport.body();
    let rows = trade_detail_rows(context, route);
    let offset = chooser.detail_scroll.min(rows.len().saturating_sub(1));
    label(
        frame,
        format!("Trade details {}/{} | PgUp/PgDn", offset + 1, rows.len()),
        body.x,
        body.y,
        Ink::Yellow,
        body,
    );
    let area = Rect::new(
        body.x,
        body.y + 20.0,
        body.width,
        (body.height - 43.0).max(0.0),
    );
    paragraphs(frame, &rows[offset..], area, 0);
    button(
        frame,
        "Back to route",
        Rect::new(body.x, body.y + body.height - 22.0, 116.0, 22.0),
        Control::Key(super::Key::Details),
        true,
        None,
    );
}

fn plot_course(frame: &mut Frame, context: &Context<'_>, area: Rect, plan: &RoutePlan) {
    let records = plan
        .systems()
        .iter()
        .filter_map(|id| context.catalogue.world(*id))
        .collect::<Vec<_>>();
    if records.len() < 2 || area.height < 24.0 {
        return;
    }
    let min_x = records
        .iter()
        .map(|world| world.position[0])
        .fold(f64::INFINITY, f64::min);
    let max_x = records
        .iter()
        .map(|world| world.position[0])
        .fold(f64::NEG_INFINITY, f64::max);
    let min_y = records
        .iter()
        .map(|world| world.position[1])
        .fold(f64::INFINITY, f64::min);
    let max_y = records
        .iter()
        .map(|world| world.position[1])
        .fold(f64::NEG_INFINITY, f64::max);
    let center = area.centre();
    let scale = ((area.width - 40.0).max(0.0) / (max_x - min_x).max(1.0))
        .min((area.height - 28.0).max(0.0) / (max_y - min_y).max(1.0));
    let points = records
        .iter()
        .map(|world| {
            [
                center[0] + (world.position[0] - (min_x + max_x) / 2.0) * scale,
                center[1] + (world.position[1] - (min_y + max_y) / 2.0) * scale,
            ]
        })
        .collect::<Vec<_>>();
    for leg in points.windows(2) {
        frame.lines.push(Line {
            from: leg[0],
            to: leg[1],
            ink: Ink::Cyan,
            clip: area,
        });
    }
    for (world, point) in records.into_iter().zip(points) {
        let x = (point[0] + 5.0).min(area.x + (area.width - 62.0).max(0.0));
        label(frame, world.name, x, point[1] - 13.0, Ink::White, area);
    }
}

fn world_name(context: &Context<'_>, id: SystemId) -> &'static str {
    context
        .catalogue
        .world(id)
        .map_or("Unknown system", |world| world.name)
}
