//! Paged read-only summaries. No buy/route transaction consumes a quoted estimate.
use super::*;
use meridian_core::NavigationTradePreview;
pub(super) struct Information {
    pub(super) preview: NavigationTradePreview,
    pages: Vec<Vec<String>>,
    page: usize,
    groups: Vec<Vec<String>>,
    viewport: ui::Viewport,
}
fn money(value: u128) -> String {
    format!("{}.{:01}", value / 10, value % 10)
}
fn signed(value: i128) -> String {
    format!(
        "{}{}",
        if value < 0 { "-" } else { "" },
        money(value.unsigned_abs())
    )
}
fn wrap(text: &str, columns: usize) -> Vec<String> {
    let mut rows = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let chars: Vec<_> = word.chars().collect();
        for part in chars.chunks(columns.max(1)) {
            let word: String = part.iter().collect();
            if !line.is_empty() && line.chars().count() + 1 + word.chars().count() > columns {
                rows.push(std::mem::take(&mut line));
            }
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(&word);
        }
    }
    if !line.is_empty() {
        rows.push(line);
    }
    rows
}
impl Information {
    fn new(preview: NavigationTradePreview, viewport: ui::Viewport) -> Result<Self, String> {
        let mut pages = vec![vec![
            format!(
                "As-of tick {}: INFORMATION ONLY; Enter/B closes",
                preview.as_of_tick
            ),
            "Only Avenor/Renas/Lareria; current asks, initial sale-bid benchmarks".into(),
            "Gross estimates exclude fuel, fees and credit caps; stock/sale may change".into(),
            "Leg opportunities are independent; no future cash/refuel/arrival promised".into(),
            "PgUp/PgDn pages. O/R separately reviews an actual navigation route".into(),
        ]];
        for route in &preview.paths {
            pages.push(vec![
                route.labels.join(" / "),
                route
                    .systems
                    .iter()
                    .map(|id| format!("G{:02}-S{:03}", id.galaxy + 1, id.system))
                    .collect::<Vec<_>>()
                    .join(" -> "),
                format!(
                    "{} jumps | {:.2} LY | arrival exposure {:.2}",
                    route.systems.len() - 1,
                    route.distance_ly,
                    route.arrival_exposure
                ),
                format!(
                    "Independent legs {} CR; best carried benchmark {} CR",
                    money(route.independent_leg_gross_tenths),
                    money(route.best_carried_sale_tenths)
                ),
                "Gross benchmarks only; later fuel, stock and sale are not guaranteed".into(),
            ]);
            for leg in &route.legs {
                let mut lines = vec![format!(
                    "Independent leg S{:03} -> S{:03}; not a funded purchase",
                    leg.source.system, leg.destination.system
                )];
                if let Some(e) = &leg.opportunity {
                    let name = e
                        .commodity
                        .definition()
                        .map_err(crate::market::content_error)?
                        .name;
                    lines.push(format!(
                        "{} {} {}; ask {} / initial bid {} CR",
                        name,
                        e.quantity,
                        e.commodity.unit().symbol(),
                        money(u128::from(e.ask_tenths)),
                        money(u128::from(e.benchmark_bid_tenths))
                    ));
                    lines.push(format!(
                        "Gross opportunity {} CR; future stock/sale not promised",
                        signed(e.gross_tenths)
                    ));
                } else {
                    lines.push("No positive independent commodity spread at these quotes".into());
                }
                pages.push(lines);
            }
        }
        if preview.direct_profits.is_empty() {
            pages.push(vec![
                "No positive direct destination at current fitted/fuel reach".into(),
                "Carried cargo excludes reserved mission/tutorial units".into(),
            ]);
        }
        for profit in &preview.direct_profits {
            let rows: Vec<_> = profit
                .exports
                .iter()
                .map(|e| {
                    let name = e
                        .commodity
                        .definition()
                        .map_err(crate::market::content_error)?
                        .name;
                    Ok(format!(
                        "{} {} {}: ask {} / initial bid {} CR; gross {}",
                        name,
                        e.quantity,
                        e.commodity.unit().symbol(),
                        money(u128::from(e.ask_tenths)),
                        money(u128::from(e.benchmark_bid_tenths)),
                        signed(e.gross_tenths)
                    ))
                })
                .collect::<Result<_, String>>()?;
            for chunk in rows.chunks(3).chain(rows.is_empty().then_some(&[][..])) {
                let mut page = vec![
                    format!(
                        "Direct G{:02}-S{:03}: delta {} CR vs selling here",
                        profit.destination.galaxy + 1,
                        profit.destination.system,
                        signed(profit.estimated_delta_tenths)
                    ),
                    format!(
                        "Sellable carried delta {} CR; exports cash/hold/stock limited",
                        signed(profit.carried_delta_tenths)
                    ),
                ];
                page.extend(chunk.iter().cloned());
                pages.push(page);
            }
        }
        let mut information = Self {
            preview,
            pages: Vec::new(),
            page: 0,
            groups: pages,
            viewport,
        };
        information.reflow(viewport);
        Ok(information)
    }
    fn reflow(&mut self, viewport: ui::Viewport) {
        if viewport == self.viewport && !self.pages.is_empty() {
            return;
        }
        self.viewport = viewport;
        let columns = ((viewport.width - 32.0) / 7.5).floor().max(1.0) as usize;
        let count = ((viewport.height - 146.0 - 64.0).max(0.0) / 18.0)
            .floor()
            .max(1.0) as usize;
        self.pages.clear();
        for group in &self.groups {
            let rows: Vec<_> = group.iter().flat_map(|line| wrap(line, columns)).collect();
            self.pages
                .extend(rows.chunks(count).map(|chunk| chunk.to_vec()));
        }
        self.page = self.page.min(self.pages.len() - 1);
    }
    fn title(&self) -> String {
        format!(
            "Trade info {}/{} | PgUp/PgDn",
            self.page + 1,
            self.pages.len()
        )
    }
    fn details(&self) -> Vec<String> {
        self.pages[self.page].clone()
    }
}
impl Docked {
    pub(super) fn reflow_trade_information(&mut self, viewport: ui::Viewport) {
        if let Some(Pending {
            intent: Intent::TradeInformation(info),
            view,
            ..
        }) = &mut self.pending
        {
            info.reflow(viewport);
            view.details = info.details();
            view.title = info.title();
        }
    }
    pub(super) fn open_trade_information(&mut self, world: &World) -> Result<(), String> {
        let preview = world
            .navigation_trade_preview(self.screens.selected_system(), world.snapshot().tick)
            .map_err(|e| e.to_string())?;
        let viewport = self
            .painted
            .as_ref()
            .ok_or("Wait for the current screen to draw")?
            .viewport;
        let info = Information::new(preview, viewport)?;
        let details = info.details();
        let title = info.title();
        self.review(
            world,
            Intent::TradeInformation(Box::new(info)),
            &title,
            details,
        )
    }
    pub(super) fn trade_information_page(&mut self, input: &ui::Input) -> Result<bool, String> {
        let change = match input {
            ui::Input::Key(ui::Key::PagePrevious) => -1,
            ui::Input::Key(ui::Key::PageNext) => 1,
            _ => return Ok(false),
        };
        let Some(Pending {
            intent: Intent::TradeInformation(info),
            view,
            ..
        }) = &mut self.pending
        else {
            return Ok(false);
        };
        if self
            .painted
            .as_ref()
            .is_none_or(|frame| frame.stamp.revision != self.serial)
        {
            return Err("Wait for the current screen to draw".into());
        }
        info.page = if change < 0 {
            info.page.saturating_sub(1)
        } else {
            (info.page + 1).min(info.pages.len() - 1)
        };
        view.details = info.details();
        view.title = info.title();
        Ok(true)
    }
}
#[cfg(test)]
mod tests;
