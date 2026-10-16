// The classic layout: MMR 1.16's main window (tabs, group boxes and their
// labels, in the same places) drawn with this app's style. Positions come
// from MMR 1.16's MainForm designer file; each control is mapped to the
// setting it changed there.

/** One control inside a group. */
export type ClassicItem =
  /** A setting: checkbox (bool), dropdown (enum, small numbers) or number box. */
  | { s: string; label?: string }
  /** One option of a flags setting, as a checkbox. */
  | { flag: string; value: string; label: string }
  /** Every option of a flags setting, as checkboxes. */
  | { flags: string }
  /** A control with its own behaviour (see ClassicCustom.svelte). */
  | { custom: ClassicCustom; label?: string; form?: string }
  /** Explanatory text. */
  | { note: string; warn?: boolean };

export type ClassicCustom =
  | "tricks"
  | "user-logic"
  | "starting-items"
  | "junk-locations"
  | "trap-weights"
  | "gossip-counts"
  | "garo-counts"
  | "hint-priorities"
  | "random-starting"
  | "form-cosmetics"
  | "hud-colors";

export interface ClassicGroup {
  /** CSS grid area in the tab. */
  area: string;
  /** Group box title; none for a plain area. */
  title?: string;
  /** Columns the controls flow into. */
  cols?: number;
  items?: ClassicItem[];
  /** Inner tabs instead of items (like "Other / Victory Mode / ..."). */
  tabs?: { title: string; cols?: number; items: ClassicItem[] }[];
}

export interface ClassicTab {
  id: string;
  title: string;
  /** grid-template-areas rows and grid-template-columns, for wide windows. */
  areas: string[];
  columns: string;
  groups: ClassicGroup[];
}

/** Flags settings that MMR 1.16 showed as inner tabs on Main Settings. */
const MODE_TABS: [string, string][] = [
  ["VictoryMode", "Victory Mode"],
  ["PriceMode", "Randomize Prices"],
  ["BossRemainsMode", "Boss Remains Mode"],
  ["BossKeyMode", "Boss Key Mode"],
  ["SmallKeyMode", "Small Key Mode"],
  ["StrayFairyMode", "Dungeon Fairy Mode"],
  ["DungeonNavigationMode", "Dungeon Navigation Mode"],
  ["EntranceMode", "Entrance Mode"],
  ["EnemyMode", "Enemy Mode"],
];

const FORMS: [string, string][] = [
  ["Human", "Human"],
  ["Deku", "Deku"],
  ["Goron", "Goron"],
  ["Zora", "Zora"],
  ["FierceDeity", "Fierce Deity"],
];

export const CLASSIC_TABS: ClassicTab[] = [
  {
    id: "main",
    title: "Main Settings",
    areas: ["gen extra", "gen junk", "other other"],
    columns: "1fr 1fr",
    groups: [
      {
        area: "gen",
        title: "Generation Settings",
        items: [
          { custom: "tricks" },
          { s: "LogicMode", label: "Mode/Logic" },
          { custom: "user-logic" },
          { s: "ItemPlacement", label: "Item Placement Order" },
        ],
      },
      {
        area: "extra",
        title: "Extra Starting Items",
        items: [{ custom: "random-starting", label: "+ Random Starting Items" }, { custom: "starting-items" }],
      },
      { area: "junk", title: "Enforce Junk Locations", items: [{ custom: "junk-locations" }] },
      {
        area: "other",
        tabs: [
          {
            title: "Other",
            cols: 3,
            items: [
              { flag: "EntranceMode", value: "DungeonEntrances", label: "Shuffle dungeon entrances" },
              { flag: "EntranceMode", value: "BossRooms", label: "Shuffle boss rooms" },
              { s: "ProgressiveUpgrades", label: "Progressive Upgrades" },
              { s: "AddSongs", label: "Mix songs with items" },
              { flag: "EnemyMode", value: "Randomized", label: "Enemies/Actors (BETA!)" },
              { s: "RandomizeGibdoRequirements", label: "Gibdo Requirements" },
              { s: "StartingItemMode", label: "Starting Items" },
              { s: "RequiredBossRemains", label: "Boss Remains For Moon" },
            ],
          },
          ...MODE_TABS.map(([key, title]) => ({ title, cols: 4, items: [{ flags: key }] })),
        ],
      },
    ],
  },
  {
    id: "items",
    title: "Item Randomization",
    areas: ["pool"],
    columns: "1fr",
    groups: [], // drawn by ClassicItemPool.svelte
  },
  {
    id: "gimmicks",
    title: "Gimmicks",
    areas: ["move warn warn other", "move traps chal other", "clock traps chal other"],
    columns: "1fr 1.1fr 1.5fr 1fr",
    groups: [
      {
        area: "move",
        title: "Movement",
        items: [
          { s: "MovementMode", label: "Gravity / Speed" },
          { s: "FloorType", label: "Floor types" },
          { s: "ContinuousDekuHopping", label: "Continuous Deku Hopping" },
          { s: "HookshotAnySurface", label: "Hookshot Any Surface" },
          { s: "ClimbMostSurfaces", label: "Climb Most Surfaces" },
          { s: "IronGoron", label: "Iron Goron" },
        ],
      },
      {
        area: "clock",
        title: "Clock",
        items: [
          { s: "ClockSpeed", label: "Speed" },
          { s: "AutoInvert", label: "Auto-invert" },
          { s: "HideClock", label: "Hide UI" },
        ],
      },
      {
        area: "warn",
        items: [
          {
            note: "WARNING! Most of these settings are not considered in logic and some can cause the seed to be unbeatable. Use at your own risk!",
            warn: true,
          },
        ],
      },
      {
        area: "traps",
        title: "Traps",
        items: [
          { s: "TrapAmount", label: "Trap Amount" },
          { s: "TrapAppearance", label: "Appearance" },
          { s: "TrapQuirks", label: "Enable Quirks" },
          { custom: "trap-weights", label: "Trap Weightings" },
        ],
      },
      {
        area: "chal",
        title: "Challenges",
        cols: 2,
        items: [
          { s: "DamageMode", label: "Damage mode" },
          { s: "DamageEffect", label: "Damage effects" },
          { s: "DeathMode", label: "Death mode" },
          { s: "FewerHealthDrops", label: "Fewer Health Drops" },
          { s: "MoonCrashErasesFile", label: "Moon Crash Erases File" },
          { s: "ByoAmmo", label: "BYO Ammo" },
          { note: "More Damage Sources" },
          { s: "TakeDamageOnEpona", label: "On Epona" },
          { s: "TakeDamageFromGibdosFaster", label: "From Gibdos Faster" },
          { s: "TakeDamageWhileShielding", label: "While Shielding" },
          { s: "TakeDamageGettingCaught", label: "Getting Caught" },
          { s: "TakeDamageFromVoid", label: "From Void" },
          { s: "TakeDamageFromDog", label: "From Dog" },
          { s: "TakeDamageFromGorons", label: "From Gorons" },
          { s: "TakeDamageFromDexihands", label: "From Dexihands" },
        ],
      },
      {
        area: "other",
        title: "Other",
        items: [
          { s: "BlastMaskCooldown", label: "Blast Mask Cooldown" },
          { s: "NutandStickDrops", label: "Nut and Stick Drops" },
          { s: "OcarinaUnderwater", label: "Underwater Ocarina" },
          { s: "EnableSunsSong", label: "Enable Sun's Song" },
          { s: "FreeScarecrow", label: "Free Scarecrow's Song" },
          { s: "AllowFierceDeityAnywhere", label: "Fierce Deity's Mask anywhere" },
          { s: "GiantMaskAnywhere", label: "Giant's Mask anywhere" },
          { s: "InstantTransform", label: "Instant Transformations" },
          { s: "BombArrows", label: "Bomb Arrows" },
          { s: "VanillaMoonTrialAccess", label: "Vanilla Moon Trial Access" },
        ],
      },
    ],
  },
  {
    id: "comfort",
    title: "Comfort",
    areas: ["speed speed comfort", "gossip garo comfort", "hints cos comfort"],
    columns: "1fr 1fr 1.6fr",
    groups: [
      {
        area: "speed",
        title: "Speed Ups",
        cols: 3,
        items: [
          { s: "SpeedupDampe", label: "Good Dampe RNG" },
          { s: "SpeedupBeavers", label: "Skip Younger Beaver" },
          { s: "SpeedupBabyCuccos", label: "Baby Cuccos Map" },
          { s: "SpeedupLabFish", label: "Faster Lab Fish" },
          { s: "SpeedupDogRace", label: "Good Dog Race RNG" },
          { s: "SpeedupBank", label: "Faster Bank" },
          { s: "DoubleArcheryRewards", label: "Double Archery Rewards" },
          { s: "RequiredZoraEggs", label: "Zora Eggs Required" },
        ],
      },
      {
        area: "gossip",
        title: "Gossip Stone Hints",
        items: [
          { s: "GossipHintStyle", label: "Hint Distribution" },
          { s: "FreeHints", label: "Free hints" },
          { s: "ClearHints", label: "Clear hints" },
          { s: "ImportanceCount", label: "Importance Count" },
          { custom: "gossip-counts" },
        ],
      },
      {
        area: "garo",
        title: "Garo Hints",
        items: [
          { s: "GaroHintStyle", label: "Hint Distribution" },
          { s: "FreeGaroHints", label: "Free hints" },
          { s: "ClearGaroHints", label: "Clear hints" },
          { s: "ImportanceCountGaro", label: "Importance Count" },
          { custom: "garo-counts" },
        ],
      },
      {
        area: "hints",
        title: "Hints",
        cols: 2,
        items: [
          { s: "HintsIndicateImportance", label: "Indicate Importance" },
          { s: "RemainsHint", label: "Boss Remains" },
          { s: "MixGossipAndGaroHints", label: "Mix Gossip and Garo Hints" },
          { s: "OathHint", label: "Oath" },
          { custom: "hint-priorities", label: "Customize Hint Priorities" },
          { s: "FairyAndSkullHint", label: "Fairies and Skulls" },
        ],
      },
      { area: "cos", title: "Cosmetic Customization", items: [{ s: "Character", label: "Player model" }] },
      {
        area: "comfort",
        title: "Comfort Options",
        cols: 2,
        items: [
          { s: "CritWiggleDisable", label: "Disable crit wiggle" },
          { s: "UpdateShopAppearance", label: "Update shops" },
          { s: "QuickTextEnabled", label: "Quick text" },
          { s: "UpdateChests", label: "Update chests" },
          { s: "FastPush", label: "Increase push speed" },
          { s: "UpdateWorldModels", label: "Update world models" },
          { s: "CloseCows", label: "Close Cows" },
          { s: "UpdateNPCText", label: "Update NPC Text" },
          { s: "EnablePictoboxSubject", label: "Improved Pictobox" },
          { s: "PreventDowngrades", label: "No downgrades" },
          { s: "LenientGoronSpikes", label: "Lenient Goron Spikes" },
          { s: "FixEponaSword", label: "Fix Epona sword" },
          { s: "TargetHealthBar", label: "Target Health Bar" },
          { s: "QuestItemStorage", label: "Quest item extra storage" },
          { s: "FillWallet", label: "Fill wallet on upgrade" },
          { s: "KeepQuestTradeThroughTime", label: "Quest items through time" },
          { s: "HiddenRupeesSparkle", label: "Hidden Rupees Sparkle" },
          { s: "ArrowCycling", label: "Arrow cycling" },
          { s: "MinorDropSparkle", label: "Minor Drops Sparkle" },
          { s: "ElegySpeedup", label: "Elegy speedups" },
          { s: "SaferGlitches", label: "Safer Glitches" },
          { s: "BombchuDrops", label: "Add Bombchu Drops" },
          { s: "EasyFrameByFrame", label: "Easy Frame By Frame" },
          { s: "KegDrops", label: "Add Keg Drops" },
          { s: "TolerantGossipStones", label: "Tolerant Gossip Angle" },
          { s: "FairyMaskShimmer", label: "Detect Stray Fairies" },
          { s: "ChestGameMinimap", label: "Treasure Chest Game Spoiler" },
          { s: "SkulltulaTokenSounds", label: "Detect Skulltula Tokens" },
        ],
      },
    ],
  },
  {
    id: "cutscenes",
    title: "Shorten Cutscenes",
    areas: ["cs"],
    columns: "1fr",
    groups: [
      {
        area: "cs",
        tabs: [
          { title: "General", cols: 5, items: [{ flags: "ShortenCutsceneSettings.General" }] },
          { title: "Boss Intros", cols: 5, items: [{ flags: "ShortenCutsceneSettings.BossIntros" }] },
        ],
      },
    ],
  },
  {
    id: "cosmetics",
    title: "Cosmetics",
    areas: ["forms other", "music hud"],
    columns: "1fr 1.3fr",
    groups: [
      {
        area: "forms",
        tabs: FORMS.map(([form, title]) => ({ title, items: [{ custom: "form-cosmetics", form }] })),
      },
      {
        area: "other",
        title: "Other",
        cols: 2,
        items: [
          { s: "Cosmetic.TatlColorSchema", label: "Tatl theme" },
          { s: "Cosmetic.BombTrapsRandomizeTunicColor", label: "Bomb Traps Randomize Tunic Color" },
          { s: "Cosmetic.CameraStyle", label: "Camera style" },
          { s: "Cosmetic.EnableHoldZTargeting", label: "Default Hold Z-Target" },
          { s: "Cosmetic.KeepPictoboxAntialiasing", label: "Keep Pictobox Antialiasing" },
          { s: "Cosmetic.RainbowTunic", label: "Rainbow Tunic" },
        ],
      },
      {
        area: "music",
        title: "Music / Sound",
        cols: 2,
        items: [
          { s: "Cosmetic.RandomizeSounds", label: "Randomize SFX" },
          { s: "Cosmetic.RemoveMinorMusic", label: "Remove Minor Music" },
          { s: "Cosmetic.EnableNightBGM", label: "Enable Night BGM" },
          { s: "Cosmetic.ShowTrackName", label: "Show Track Names" },
          { s: "Cosmetic.DisableCombatMusic", label: "Disable Combat Music" },
          { s: "Cosmetic.DisableFanfares", label: "Disable Fanfares" },
          { s: "Cosmetic.Music", label: "Music" },
          { s: "Cosmetic.MusicLuckRollChance", label: "Luck Roll Chance (%)" },
          { s: "Cosmetic.LowHealthSFX", label: "Low Health SFX" },
        ],
      },
      {
        area: "hud",
        title: "HUD",
        items: [
          { s: "Cosmetic.HeartsSelection", label: "Hearts" },
          { s: "Cosmetic.MagicSelection", label: "Magic" },
          { custom: "hud-colors", label: "Customize" },
          { s: "Cosmetic.ShiftHueMiscUI", label: "Randomize Hue of Miscellaneous UI" },
        ],
      },
    ],
  },
];

/** "DirectToCredits" -> "Direct To Credits", as MMR 1.16 labelled options. */
export function addSpaces(s: string): string {
  return s.replace(/([a-z0-9])([A-Z])/g, "$1 $2").replace(/([A-Z])([A-Z][a-z])/g, "$1 $2");
}
