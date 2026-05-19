// Sindri icons. Hairline geometry, no fills. 14px default.

const Icon = ({ d, size = 14, stroke = "currentColor", sw = 1.4, children, ...rest }) => (
  <svg width={size} height={size} viewBox="0 0 16 16" fill="none"
       stroke={stroke} strokeWidth={sw} strokeLinecap="square" strokeLinejoin="miter" {...rest}>
    {d ? <path d={d} /> : children}
  </svg>
);

const IconPlay     = (p) => <Icon {...p}><path d="M5 3 L13 8 L5 13 Z" /></Icon>;
const IconPause    = (p) => <Icon {...p}><path d="M5 3 V13 M11 3 V13" /></Icon>;
const IconStop     = (p) => <Icon {...p}><path d="M4 4 H12 V12 H4 Z" /></Icon>;
const IconStep     = (p) => <Icon {...p}><path d="M4 3 L10 8 L4 13 Z M12 3 V13" /></Icon>;
const IconMove     = (p) => <Icon {...p}><path d="M8 2 V14 M2 8 H14 M5 5 L8 2 L11 5 M5 11 L8 14 L11 11 M5 5 L2 8 L5 11 M11 5 L14 8 L11 11" sw={1.2}/></Icon>;
const IconRotate   = (p) => <Icon {...p}><path d="M3 8 A5 5 0 0 1 13 8 M11 4 L13 4 L13 6 M13 8 A5 5 0 0 1 3 8 M5 12 L3 12 L3 10"/></Icon>;
const IconScale    = (p) => <Icon {...p}><path d="M3 13 L13 3 M3 13 V8 M3 13 H8 M13 3 V8 M13 3 H8" sw={1.2}/></Icon>;
const IconUndo     = (p) => <Icon {...p}><path d="M6 4 L3 7 L6 10 M3 7 H10 A4 4 0 0 1 14 11 V12"/></Icon>;
const IconRedo     = (p) => <Icon {...p}><path d="M10 4 L13 7 L10 10 M13 7 H6 A4 4 0 0 0 2 11 V12"/></Icon>;
const IconChevron  = (p) => <Icon {...p}><path d="M5 6 L8 9 L11 6"/></Icon>;
const IconChevronR = (p) => <Icon {...p}><path d="M6 4 L9 8 L6 12"/></Icon>;
const IconPlus     = (p) => <Icon {...p}><path d="M8 3 V13 M3 8 H13"/></Icon>;
const IconX        = (p) => <Icon {...p}><path d="M4 4 L12 12 M12 4 L4 12"/></Icon>;
const IconCheck    = (p) => <Icon {...p}><path d="M3 8 L7 12 L13 4"/></Icon>;
const IconSearch   = (p) => <Icon {...p}><circle cx="7" cy="7" r="4"/><path d="M10 10 L14 14"/></Icon>;
const IconSparkle  = (p) => <Icon {...p}><path d="M8 2 L9 6 L13 7 L9 8 L8 13 L7 8 L3 7 L7 6 Z" sw={1.1}/></Icon>;
const IconSparkSmall = (p) => <Icon {...p}><path d="M8 3 L9 7 L13 8 L9 9 L8 13 L7 9 L3 8 L7 7 Z" sw={1}/></Icon>;
const IconCommand  = (p) => <Icon {...p}>
  <path d="M5 5 H11 V11 H5 Z"/>
  <path d="M5 5 A2 2 0 1 1 5 1 H5 V5 Z M11 5 A2 2 0 1 0 11 1 H11 V5 Z M5 11 A2 2 0 1 0 5 15 H5 V11 Z M11 11 A2 2 0 1 1 11 15 H11 V11 Z"/>
</Icon>;
const IconFolder   = (p) => <Icon {...p}><path d="M2 4 H6 L8 6 H14 V13 H2 Z"/></Icon>;
const IconFile     = (p) => <Icon {...p}><path d="M4 2 H10 L13 5 V14 H4 Z M10 2 V5 H13"/></Icon>;
const IconScript   = (p) => <Icon {...p}><path d="M4 2 H12 V14 H4 Z M6 5 H10 M6 8 H10 M6 11 H8"/></Icon>;
const IconSprite   = (p) => <Icon {...p}><rect x="3" y="3" width="10" height="10"/><path d="M3 10 L6 8 L9 11 L13 7"/></Icon>;
const IconTransform = (p) => <Icon {...p}><circle cx="8" cy="8" r="1.5"/><path d="M8 2 V5 M8 11 V14 M2 8 H5 M11 8 H14"/></Icon>;
const IconCamera   = (p) => <Icon {...p}><rect x="2" y="5" width="9" height="7"/><path d="M11 7 L14 5 V12 L11 10 Z"/></Icon>;
const IconAudio    = (p) => <Icon {...p}><path d="M3 6 H5 L8 3 V13 L5 10 H3 Z M10 6 A3 3 0 0 1 10 10"/></Icon>;
const IconRigid    = (p) => <Icon {...p}><rect x="3" y="3" width="10" height="10"/><circle cx="8" cy="8" r="2"/></Icon>;
const IconCollider = (p) => <Icon {...p}><path d="M3 3 H13 V13 H3 Z" strokeDasharray="2 2"/></Icon>;
const IconLight    = (p) => <Icon {...p}><circle cx="8" cy="7" r="3"/><path d="M8 11 V14 M5 14 H11 M8 1 V3 M2 7 H4 M12 7 H14 M3 3 L4 4 M13 3 L12 4"/></Icon>;
const IconTilemap  = (p) => <Icon {...p}><path d="M2 2 H7 V7 H2 Z M9 2 H14 V7 H9 Z M2 9 H7 V14 H2 Z M9 9 H14 V14 H9 Z"/></Icon>;
const IconRevert   = (p) => <Icon {...p}><path d="M3 5 L6 2 V4 H10 A4 4 0 0 1 10 12 H4"/></Icon>;
const IconChat     = (p) => <Icon {...p}><path d="M2 3 H14 V11 H6 L3 14 V11 H2 Z M5 7 H11"/></Icon>;
const IconGrid     = (p) => <Icon {...p}><path d="M2 2 H14 V14 H2 Z M6 2 V14 M10 2 V14 M2 6 H14 M2 10 H14"/></Icon>;
const IconHistory  = (p) => <Icon {...p}><path d="M3 8 A5 5 0 1 1 8 13 M3 8 H1 M3 8 L5 6 M8 5 V8 L10 10"/></Icon>;
const IconSettings = (p) => <Icon {...p}><circle cx="8" cy="8" r="2"/><path d="M8 1 V3 M8 13 V15 M1 8 H3 M13 8 H15 M3 3 L4.5 4.5 M11.5 11.5 L13 13 M3 13 L4.5 11.5 M11.5 4.5 L13 3"/></Icon>;
const IconKey      = (p) => <Icon {...p}><circle cx="5" cy="8" r="3"/><path d="M8 8 H14 V11 M11 8 V11"/></Icon>;

const COMPONENT_ICONS = {
  "Transform": IconTransform,
  "Sprite": IconSprite,
  "Script": IconScript,
  "Camera": IconCamera,
  "Audio": IconAudio,
  "RigidBody": IconRigid,
  "Collider": IconCollider,
  "Light": IconLight,
  "Tilemap": IconTilemap,
};

const ENTITY_ICONS = {
  "actor":  IconSprite,
  "static": IconGrid,
  "camera": IconCamera,
  "light":  IconLight,
};

Object.assign(window, {
  Icon, IconPlay, IconPause, IconStop, IconStep, IconMove, IconRotate, IconScale,
  IconUndo, IconRedo, IconChevron, IconChevronR, IconPlus, IconX, IconCheck,
  IconSearch, IconSparkle, IconSparkSmall, IconCommand, IconFolder, IconFile,
  IconScript, IconSprite, IconTransform, IconCamera, IconAudio, IconRigid,
  IconCollider, IconLight, IconTilemap, IconRevert, IconChat, IconGrid,
  IconHistory, IconSettings, IconKey, COMPONENT_ICONS, ENTITY_ICONS,
});
