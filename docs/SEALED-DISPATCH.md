# G01 Sealed Dispatch

At a main orbital, press **5** for Equipment. Use **]** to reach **G01 sealed dispatch board**. The three package rows show the sender, route and fee. Choose a row with **Up** or **Down**. Press **Enter** to review it. **B** goes back. Confirming with **Enter** puts one original sealed record aboard and saves the game. It uses one free hold tonne. The sender charges no deposit and sets no deadline.

Open the galaxy chart with **1** and select the next quoted stop. Review the direct route and confirm it before launching. Fly each leg, paying the ordinary fuel cost. At the final main orbital, enter through the doorway. Open the dispatch board and review **Receiving officer / hand over original**. **Enter** transfers that record, releases its tonne and credits the displayed fee. The first delivery adds the **Sealed Dispatch Family Trace** to the **Shift+J** journal. Each new job can pay its own fee.

On the first Avenor board with the stock ship and a full tank, packages 1 and 2 go to Renas and pay 410 CR. Package 3 goes through Renas to Lareria and pays 500 CR. The two jumps use 34 and 73 fuel units. Renas can sell fuel before the second leg. Prices, stock and the wallet remain under ordinary station control.

To abandon a job, return to its issuing main orbital, review **Sender / abandon for zero fee** and confirm. Abandonment releases the tonne and pays zero. A destroyed carrier loses its record. The player can save that terminal state. Continue restores the original issue, carrier, route and outcome. An unsettled record earns no fee or clue.

## Source and scope

The original closed G01 pack is `l337.contract/sealed-dispatch` version 1.0.0, digest `8cd3f7b667d3842cbe2c0f4a6acb9d304fc4bede2fce66573169569e2ed4fcb3`. The first route generator in Meridian uses the three main orbitals with ordinary station services. It applies the original breadth-first search and destination ordering to that graph. The board key, six draws, package rows 1, 3 and 5, hazard tier zero and fee of 320 CR plus 90 CR per quoted leg follow the original rules. The original Game searched a larger galaxy and supported additional sender sites. This preview's frozen route graph keeps existing consignment terms readable if more stations open later. [Pinned legacy files](SEALED-DISPATCH-SOURCES.json)

The account revision and issuing berth identify the original item for its owner and ship. Each delivery or abandonment names that issue. Reception checks the actual jump journal, physical destination berth and current receiver. The station saves the proposed account and receipt before changing the live state. Offer reviews are local and expire when the player reloads. The original route catalogue, terminal settlement and paid fuel history remain available to the save validator.
