# Station launch and docking presentation

The verified flight preview shows a ship and fixture rings but has no station gameplay loop. The next slice needs the original SH028 station display, an explicit launch control and presentation that exposes the World-owned station state.

## Acceptance

Start docked with a visible original station and launch with L. Fly with the existing controls and return through the physical entrance. World alone admits docking or refuses an unsafe crossing. Refused commands remain visible without terminating the application. Preserve pause, focus release, resize and fullscreen repaint behavior.

## Scope and risks

The station uses its complete original display geometry and separately declared native collision LOD. This is not a claim of full visual/collision equivalence. Desktop cameras are presentation choices and cannot grant launch, docking or clearance. Actual compiler, graphics and interactive loop checks belong to the Dell lane.
