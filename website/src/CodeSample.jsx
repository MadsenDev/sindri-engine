// Lua code sample with lightweight syntax highlighting.
// The base script is an excerpt from examples/scripted_asteroids/scripts/player.lua.

const LUA_SAMPLE_LINES = [
  { text: "-- examples/scripted_asteroids/scripts/player.lua" },
  { text: "local screen_w = params.screen_w or 1280.0" },
  { text: "local screen_h = params.screen_h or 720.0" },
  { text: "local rotation_speed = params.rotation_speed or 5.0" },
  { text: "local thrust = params.thrust or 260.0" },
  { text: "local friction = params.friction or 0.985" },
  { text: "local max_speed = params.max_speed or 340.0" },
  { text: "" },
  { text: "local vx = 0.0" },
  { text: "local vy = 0.0" },
  { text: "local rotation = 0.0" },
  { text: "" },
  { text: "function on_update(self, dt)" },
  { text: "    local input = self:input()" },
  { text: "    local transform = self:transform()" },
  { text: "    if transform == nil then" },
  { text: "        return" },
  { text: "    end" },
  { text: "" },
  { text: "    if input:is_key_down(\"A\") or input:is_key_down(\"Left\") then" },
  { text: "        rotation = rotation - rotation_speed * dt" },
  { text: "    end" },
  { text: "    if input:is_key_down(\"D\") or input:is_key_down(\"Right\") then" },
  { text: "        rotation = rotation + rotation_speed * dt" },
  { text: "    end" },
  { text: "    if input:is_key_down(\"W\") or input:is_key_down(\"Up\") then" },
  { text: "        vx = vx + math.cos(rotation) * thrust * dt" },
  { text: "        vy = vy + math.sin(rotation) * thrust * dt" },
  { text: "    end" },
  { text: "" },
  { text: "    local pos = transform:position()" },
  { text: "    local x = pos.x + vx * dt" },
  { text: "    local y = pos.y + vy * dt" },
  { text: "" },
  { text: "    -- ai proposal · valid sindri Lua APIs", add: true },
  { text: "    if input:is_key_pressed(\"Space\") then", add: true },
  { text: "        self:world():spawn_dynamic(", add: true },
  { text: "            transform:position(),", add: true },
  { text: "            vec2(math.cos(rotation) * 520.0, math.sin(rotation) * 520.0)", add: true },
  { text: "        )", add: true },
  { text: "    end", add: true },
  { text: "" },
  { text: "    transform:set_position(vec2(x, y))" },
  { text: "    transform:set_rotation(rotation)" },
  { text: "end" },
];

function tokenizeLua(line) {
  const parts = [];
  const pattern = /(--.*$)|("(?:[^"\\]|\\.)*")|\b(function|local|if|then|end|return|or|and|nil|not)\b|\b(\d+(?:\.\d+)?)\b|([A-Za-z_][A-Za-z0-9_]*)|(\S)|(\s+)/g;
  let match;
  while ((match = pattern.exec(line)) !== null) {
    if (match[1]) parts.push(["com", match[1]]);
    else if (match[2]) parts.push(["str", match[2]]);
    else if (match[3]) parts.push(["key", match[3]]);
    else if (match[4]) parts.push(["num", match[4]]);
    else if (match[5]) parts.push([["self", "params", "math", "vec2"].includes(match[5]) ? "prop" : "fn", match[5]]);
    else if (match[6]) parts.push(["op", match[6]]);
    else parts.push(["plain", match[7]]);
  }
  return parts;
}

function CodeSample() {
  return (
    <section id="code" className="code-section section">
      <span className="section-label">
        <span className="dot" />
        <span className="num">06</span> · scripting · plain lua, hot reload
      </span>

      <div className="wrap">
        <div className="section-heading">
          <div className="left">
            <span className="eyebrow muted"><span className="bar" />gameplay layer · real api</span>
            <h2>Behaviour is plain Lua.<br />Hot reload, deferred mutation.</h2>
          </div>
          <div className="right">
            <p>
              Sindri scripts attach to entities, hook lifecycle callbacks such as <code>on_start</code>,
              <code>on_update</code>, <code>on_fixed_update</code>, collision, and trigger events, then mutate
              the world through the deferred command buffer. The base excerpt below follows
              <code>examples/scripted_asteroids/scripts/player.lua</code>; the highlighted proposal only uses APIs
              exposed by <code>crates/sindri/src/script.rs</code>.
            </p>
          </div>
        </div>

        <div className="code-block">
          <div className="lhs">
            <div className="code-head">
              <IconScript size={11} />
              <span className="file">examples/scripted_asteroids/scripts/player.lua</span>
              <span className="lang"><span className="amber">+ proposed write_script block</span> · lua 5.4</span>
            </div>
            <div className="code-body">
              {LUA_SAMPLE_LINES.map((line, i) => {
                const toks = tokenizeLua(line.text);
                return (
                  <div key={i} className={"code-line" + (line.add ? " add" : "")}>
                    <span className="ln">{i + 1}</span>
                    <span>
                      {line.text.length === 0 && "\u00A0"}
                      {toks.map(([k, text], j) => (
                        k === "plain"
                          ? <span key={j}>{text.replace(/ /g, "\u00A0")}</span>
                          : <span key={j} className={"tk-" + k}>{text}</span>
                      ))}
                    </span>
                  </div>
                );
              })}
            </div>
          </div>
          <div className="code-rhs">
            <span className="eyebrow"><span className="bar"/>real api · illustrative diff</span>
            <h3>Lifecycle, facets, command buffer</h3>
            <p>
              The shown calls exist today: <code>self:input()</code>, <code>self:transform()</code>,
              <code>self:world()</code>, <code>transform:position()</code>, <code>transform:set_position()</code>,
              and <code>world:spawn_dynamic()</code>. There is no fictional audio facet or made-up component type
              in this sample.
            </p>
            <div className="kv">
              <div className="row"><span className="k">runtime</span><span className="v">mlua 0.9 · Lua 5.4</span></div>
              <div className="row"><span className="k">lifecycle</span><span className="v">on_start · on_update · on_fixed_update</span></div>
              <div className="row"><span className="k">events</span><span className="v">collision · trigger · destroy</span></div>
              <div className="row"><span className="k">mutation</span><span className="v">deferred command buffer</span></div>
              <div className="row"><span className="k">script params</span><span className="v">params table per attachment</span></div>
              <div className="row"><span className="k">ai action</span><span className="v amber">write_script</span></div>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}

Object.assign(window, { CodeSample });
