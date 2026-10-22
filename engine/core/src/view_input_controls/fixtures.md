# Independent expected fixtures (written before implementation)

Source: published view-input-controls.md sections 2-8/10. Values below are
hand-derived; tests must not derive their expectations using the owner helpers.

* Scroll anchor (100,100), default thresholds (12,80): mouse (124,100) gives
  raw (12,0), stored speed (0,0), neutral/off; (126,124) gives raw (13,12),
  stored (13,0), southeast cursor 0x22A676A0 (inclusive cursor boundary),
  magnitude sqrt(313)/68, angle atan(12/13). (300,100) stores (80,0) but
  speed is 1 from raw (100,0). (99,99) truncates to zero on both axes.
  (74,74) gives northwest 0x02A67691 and angle 5*pi/4. Physical release
  stops scrolling, deletes anchor child, then removes top(false), handled=true.
* Level cells (3,4) -> (1,6), city 8x8: mark inclusive (1,4,3,6),
  vertex rectangle (1,4,4,7), height from index 39, not rectangle origin 37.
  City 4x6 clamps vertex maximum to (4,6). Miss on release still commits last
  endpoint. Press miss is handled, no capture; move/release idle are false.
* Road edges W,N,E,S = [0,2,0,2]: orientation 0, no offset; street height
  .64. [1,3,0,0]: orientation 2, offset (-4,-4). [2,1,2,0] must take
  W/E precedence: orientation 1, zero offset. [0,0,0,2]: orientation 0.
  [0,2,0,0] with rail+road flags uses rail edges and height .8.
* Avenue [1,1,1,1]: orientation 2, zero offset (four diagonal branch).
  [3,0,0,0]: orientation 3, offset (-8,8). [2,1,1,2]: orientation 2,
  offset (-8,8). [4,2,0,0]: orientation 0, offset (-8,0).
  Missing avenue piece preserves exact input position and orientation 1.
* Water wins over networks: colour (.5,.5,1), height 2, exact input,
  current orientation. No network: (.5,1,.5), height 2, exact input.
  Road snap keeps input y; Home mapping [3,2,0,1], End [2,3,1,0].
* Label selected: new alpha .7 BEFORE old alpha .5. Press/move changes
  pressed to dragging, dynamic=true, uses mouse hit without stored offset.
  Failed edit on release: alpha .5, selection cleared, remove; idle.
  Modal result OK+empty fails, cancel fails, unknown result keeps old text.
* Sign selection: old empty removed, old nonempty dynamic=false, then new
  dynamic=true. New placement y=max(ground,sea). A mere press/release enters
  typing; nonempty drag releases to idle, empty drag enters typing.
* Sign 32 glyphs + one glyph -> 33; next append ignored. Text length 31,
  paste newline,A,B -> newline consumes slot 32, A produces length 32,
  B ignored. Backspace removes one complete multibyte glyph. Escape restores
  backup, remains typing and returns false. Enter deletes an empty selection.
  Ctrl+V only when (modifiers & 7)==2. Position reply uses bounds centre x/z,
  second corner y, core view-utility projection; never consumes message.
* Filters: label requires prop AND decal interface, sign requires prop AND
  sign interface; demolition requires prop AND either exact type id, regardless
  of interface. Renderer miss falls back to terrain then first occupant.
  Demolition selection transitions unhighlight old before highlight new;
  remove drops selection without touching deleted prop, sound 0x8A5EC047.
