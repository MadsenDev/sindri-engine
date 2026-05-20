// Sindri icons — hairline 16x16 geometry, stroke-only.
// Mirrors the design system's ref/js/icons.jsx; trimmed to what the site uses.

const Icon = ({ d, size = 14, stroke = "currentColor", sw = 1.4, children, style, ...rest }) => (
  <svg width={size} height={size} viewBox="0 0 16 16" fill="none"
       stroke={stroke} strokeWidth={sw} strokeLinecap="square" strokeLinejoin="miter"
       style={{ display: "block", ...style }} {...rest}>
    {d ? <path d={d} /> : children}
  </svg>
);

const IconPlay     = (p) => <Icon {...p}><path d="M5 3 L13 8 L5 13 Z" /></Icon>;
const IconPause    = (p) => <Icon {...p}><path d="M5 3 V13 M11 3 V13" /></Icon>;
const IconStop     = (p) => <Icon {...p}><path d="M4 4 H12 V12 H4 Z" /></Icon>;
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
  <path d="M5 5 A2 2 0 1 1 5 1 V5 Z M11 5 A2 2 0 1 0 11 1 V5 Z M5 11 A2 2 0 1 0 5 15 V11 Z M11 11 A2 2 0 1 1 11 15 V11 Z"/>
</Icon>;
const IconFolder   = (p) => <Icon {...p}><path d="M2 4 H6 L8 6 H14 V13 H2 Z"/></Icon>;
const IconScript   = (p) => <Icon {...p}><path d="M4 2 H12 V14 H4 Z M6 5 H10 M6 8 H10 M6 11 H8"/></Icon>;
const IconSprite   = (p) => <Icon {...p}><rect x="3" y="3" width="10" height="10"/><path d="M3 10 L6 8 L9 11 L13 7"/></Icon>;
const IconTransform = (p) => <Icon {...p}><circle cx="8" cy="8" r="1.5"/><path d="M8 2 V5 M8 11 V14 M2 8 H5 M11 8 H14"/></Icon>;
const IconCamera   = (p) => <Icon {...p}><rect x="2" y="5" width="9" height="7"/><path d="M11 7 L14 5 V12 L11 10 Z"/></Icon>;
const IconAudio    = (p) => <Icon {...p}><path d="M3 6 H5 L8 3 V13 L5 10 H3 Z M10 6 A3 3 0 0 1 10 10"/></Icon>;
const IconRigid    = (p) => <Icon {...p}><rect x="3" y="3" width="10" height="10"/><circle cx="8" cy="8" r="2"/></Icon>;
const IconCollider = (p) => <Icon {...p}><path d="M3 3 H13 V13 H3 Z" strokeDasharray="2 2"/></Icon>;
const IconLight    = (p) => <Icon {...p}><circle cx="8" cy="7" r="3"/><path d="M8 11 V14 M5 14 H11 M8 1 V3 M2 7 H4 M12 7 H14 M3 3 L4 4 M13 3 L12 4"/></Icon>;
const IconTilemap  = (p) => <Icon {...p}><path d="M2 2 H7 V7 H2 Z M9 2 H14 V7 H9 Z M2 9 H7 V14 H2 Z M9 9 H14 V14 H9 Z"/></Icon>;
const IconGrid     = (p) => <Icon {...p}><path d="M2 2 H14 V14 H2 Z M6 2 V14 M10 2 V14 M2 6 H14 M2 10 H14"/></Icon>;
const IconHistory  = (p) => <Icon {...p}><path d="M3 8 A5 5 0 1 1 8 13 M3 8 H1 M3 8 L5 6 M8 5 V8 L10 10"/></Icon>;
const IconSettings = (p) => <Icon {...p}><circle cx="8" cy="8" r="2"/><path d="M8 1 V3 M8 13 V15 M1 8 H3 M13 8 H15 M3 3 L4.5 4.5 M11.5 11.5 L13 13 M3 13 L4.5 11.5 M11.5 4.5 L13 3"/></Icon>;
const IconChat     = (p) => <Icon {...p}><path d="M2 3 H14 V11 H6 L3 14 V11 H2 Z M5 7 H11"/></Icon>;
const IconCpu      = (p) => <Icon {...p}><rect x="4" y="4" width="8" height="8"/><rect x="6" y="6" width="4" height="4"/><path d="M8 2 V4 M8 12 V14 M2 8 H4 M12 8 H14 M5 2 V4 M11 2 V4 M5 12 V14 M11 12 V14 M2 5 H4 M2 11 H4 M12 5 H14 M12 11 H14"/></Icon>;
const IconBranch   = (p) => <Icon {...p}><circle cx="5" cy="4" r="1.5"/><circle cx="5" cy="12" r="1.5"/><circle cx="12" cy="6" r="1.5"/><path d="M5 5.5 V10.5 M5 10 C 5 8 6 6 10.5 6"/></Icon>;
const IconBolt     = (p) => <Icon {...p}><path d="M9 1 L4 9 H8 L7 15 L12 7 H8 Z"/></Icon>;
const IconDownload = (p) => <Icon {...p}><path d="M8 2 V11 M4 7 L8 11 L12 7 M3 14 H13"/></Icon>;
const IconExternal = (p) => <Icon {...p}><path d="M6 3 H3 V13 H13 V10 M9 3 H13 V7 M13 3 L8 8"/></Icon>;
const IconGithub   = (p) => <Icon {...p}><path d="M8 2 C 4.5 2 2 4.5 2 8 C 2 10.8 3.8 13.2 6.3 14 L 6.3 12.5 C 4.5 12.9 4.1 11.5 4.1 11.5 C 3.8 10.8 3.4 10.6 3.4 10.6 C 2.8 10.2 3.5 10.2 3.5 10.2 C 4.2 10.2 4.5 10.9 4.5 10.9 C 5.1 12 6.2 11.7 6.6 11.5 C 6.7 11 6.9 10.7 7.1 10.5 C 5.5 10.3 4 9.7 4 7.2 C 4 6.5 4.3 5.9 4.7 5.5 C 4.7 5.3 4.4 4.7 4.8 3.7 C 4.8 3.7 5.3 3.5 6.5 4.4 C 7 4.3 7.5 4.2 8 4.2 C 8.5 4.2 9 4.3 9.5 4.4 C 10.7 3.5 11.2 3.7 11.2 3.7 C 11.6 4.7 11.3 5.3 11.3 5.5 C 11.7 5.9 12 6.5 12 7.2 C 12 9.7 10.5 10.3 8.9 10.5 C 9.1 10.7 9.4 11.1 9.4 11.8 L 9.4 14 C 12 13.2 13.8 10.8 13.8 8 C 14 4.5 11.5 2 8 2 Z"/></Icon>;
const IconBook     = (p) => <Icon {...p}><path d="M2 3 H7 C 7.5 3 8 3.5 8 4 V13 C 8 12.5 7.5 12 7 12 H2 Z M14 3 H9 C 8.5 3 8 3.5 8 4 V13 C 8 12.5 8.5 12 9 12 H14 Z"/></Icon>;
const IconRefine   = (p) => <Icon {...p}><path d="M2 8 H10 M7 5 L10 8 L7 11"/></Icon>;
const IconRust     = (p) => <Icon {...p}><circle cx="8" cy="8" r="5"/><path d="M8 3 V13 M3 8 H13 M4.5 4.5 L11.5 11.5 M11.5 4.5 L4.5 11.5"/></Icon>;
const IconLua      = (p) => <Icon {...p}><circle cx="8" cy="8" r="6"/><circle cx="10.5" cy="5.5" r="1"/></Icon>;
const IconWgpu     = (p) => <Icon {...p}><path d="M2 5 L8 2 L14 5 L8 8 Z M2 5 V11 L8 14 M14 5 V11 L8 14 M8 8 V14"/></Icon>;
const IconWindows  = (p) => <Icon {...p}><path d="M2 3 L7 2 V7 H2 Z M7 2 L14 2 V7 H7 Z M2 7 H7 V13 L2 12 Z M7 7 H14 V14 L7 13 Z"/></Icon>;
const IconApple    = (p) => <Icon {...p}><path d="M11 3 C 10 4 9 5 8 4.5 C 8 3 9 2 10.5 2 M5 7 C 3 7 2 9 2 11 C 2 13 4 14 5.5 14 C 6.5 14 7 13.5 8 13.5 C 9 13.5 9.5 14 10.5 14 C 12 14 14 13 14 11 C 14 9 13 7 11 7 C 9.5 7 9 8 8 8 C 7 8 6.5 7 5 7 Z"/></Icon>;
const IconLinux    = (p) => <Icon {...p}><circle cx="8" cy="7" r="3"/><path d="M6 7 L7 8 M10 7 L9 8 M5 10 L4 14 L12 14 L11 10 M6.5 6 C 6.5 5.5 7 5 8 5 C 9 5 9.5 5.5 9.5 6"/></Icon>;

Object.assign(window, {
  Icon, IconPlay, IconPause, IconStop, IconUndo, IconRedo, IconChevron, IconChevronR,
  IconPlus, IconX, IconCheck, IconSearch, IconSparkle, IconSparkSmall, IconCommand,
  IconFolder, IconScript, IconSprite, IconTransform, IconCamera, IconAudio, IconRigid,
  IconCollider, IconLight, IconTilemap, IconGrid, IconHistory, IconSettings, IconChat,
  IconCpu, IconBranch, IconBolt, IconDownload, IconExternal, IconGithub, IconBook,
  IconRefine, IconRust, IconLua, IconWgpu, IconWindows, IconApple, IconLinux,
});
