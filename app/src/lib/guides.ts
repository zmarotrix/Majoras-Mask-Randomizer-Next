// The logic editor guides, shown in the editor's Guide panel.
//
// Bodies use a small markup (see Guide.svelte): "### " subheadings,
// "- " and "1. " lists, "> " tips, "| " tables, **bold**, `code`,
// [text](#section-id) links, and {kind:Label} for coloured tags
// (kinds: item, region, helper, trick, setting, event, location).

export interface GuideSection {
  id: string;
  title: string;
  body: string;
}

export interface Guide {
  id: "beginner" | "veteran";
  title: string;
  blurb: string;
  sections: GuideSection[];
}

const beginner: Guide = {
  id: "beginner",
  title: "Starting from scratch",
  blurb: "Never edited logic before? Start here. No programming needed.",
  sections: [
    {
      id: "what",
      title: "What logic is",
      body: `
The randomizer shuffles where items are. Before it hands you a seed, it makes sure the game can still be finished: it never hides the {item:Hero's Bow} behind a door that needs the {item:Hero's Bow}.

To do that it needs to know **what you need to reach every check in the game**. That knowledge is the *logic*. The logic editor is where you read it, change it, or write your own.

You might want to:

- **Understand a seed.** "Why did logic expect me to get this with only the {item:Deku Mask}?" Look the check up and see exactly what it needs.
- **Make seeds easier or harder.** Add a requirement (you must have {item:Bombs} before logic sends you somewhere), or remove one.
- **Add a trick.** Teach logic a technique you can do, so seeds that turn it on can expect it.
- **Fix a mistake.** If logic expects something impossible, you can correct it and share the file.

> Nothing you do here changes the game itself. You're only changing what the randomizer *assumes* you can do.
`,
    },
    {
      id: "idea",
      title: "The big idea: a map of places",
      body: `
Logic is organised like a map. There are four kinds of thing:

- **Regions** are places, like {region:South Clock Town} or {region:Woodfall Temple: Main Room}. They're the green tags.
- **Exits** connect one region to another. Each exit says what you need to go that way. Going from {region:Termina Field} into the Southern Swamp might need nothing; going into the temple needs a song and a mask.
- **Checks** are the places items can be: chests, NPC gifts, heart pieces, songs. Each check lives in a region and says what you need **once you're standing in that region**.
- **Events** are things that happen in a region that other rules care about: a boss beaten, water drained, a wall blown up. They're the pink tags, like {event:Woodfall Cleared}.

### Why this makes things simpler

Because a check is inside a region, its rule only has to say the *extra* thing it needs. A chest on a high ledge in the temple just says "needs {item:Deku Mask}". It doesn't have to repeat everything needed to get *into* the temple, because the temple's exits already say that.

So, to find out everything you need for a check, you follow the path from the start of the game to its region and add up what each exit needed, then add the check's own rule. The randomizer does that for you.

### An example

1. Everything begins in the {region:Start} region, which leads into {region:South Clock Town}.
2. An exit from town leads to {region:Termina Field}. It needs nothing.
3. An exit from {region:Termina Field} leads to {region:Southern Swamp}. It needs nothing.
4. Going further, towards Woodfall, needs the {item:Deku Mask}.
5. A heart piece in Woodfall needs nothing extra once you're there.

So the heart piece really needs the {item:Deku Mask}, because that's the only way there.
`,
    },
    {
      id: "tour",
      title: "A tour of the editor",
      body: `
Open the editor from the main window's **Logic editor** button. It opens in its own window, which you can resize or maximise.

When it opens, choose what to edit:

- **Casual logic** is the normal logic most seeds use.
- **Glitched logic** allows glitches.
- **Open or import a file** loads a logic file, including ones made for older versions of MMR (they're converted for you).

Editing the built-in logic doesn't change it for good. When you save, you save **a copy** to a file of your own.

### The three columns

- **Left: the navigator.** Search at the top. Under it are four tabs: **Regions** (grouped by area of the game), **Helpers**, **Tricks** and **Settings**. Click anything to open it. A red number means that thing has a problem.
- **Middle: the page.** The region or helper you opened. This is where you read logic.
- **Right: the inspector.** Click a row on the page (an exit, a check, an event) and its details appear here, ready to edit.

### The toolbar

- **Open** switches to other logic.
- **Undo / Redo** (Ctrl+Z / Ctrl+Y) step back and forward through every change. Hover them to see what they'll undo.
- **Problems** shows mistakes the editor has found. Click one to jump to it.
- **Save** writes your logic to a file. The arrow next to it has **Save as** and **Export for older MMR versions**.
- **Guide** opens this guide.

A dot next to the file name means you have unsaved changes.
`,
    },
    {
      id: "reading",
      title: "Reading a region page",
      body: `
A region page has four lists:

- **Ways in**: the exits from other regions that lead here, and what each needs. Click one to jump to the region it starts from.
- **Ways out**: exits from here to other regions.
- **Checks**: item locations in this region and what each needs once you're here. Times of day, if any, show as a badge like \`D1 N1\`.
- **Events**: things that happen here that other rules can need.

### Reading a requirement

Requirements are written as a short list:

- **needs** lists what's always required. Everything on this line is needed.
- **and one of** lists alternatives, numbered 1, 2, 3… You need **just one** of the numbered lines (as well as everything on the "needs" line).
- **nothing needed** means you can always do it once you're in the region.
- **impossible** means it can never be done. That's usually a mistake.

For example:

- needs {item:Bombs}
- and one of: 1. {item:Hero's Bow} · 2. {item:Hookshot}

means *bombs, plus either the bow or the hookshot*.

### Colours

Every tag is coloured by what it is:

- {item:Item}: something you collect.
- {region:Region}: you must be able to reach that place.
- {event:Event}: something that must have happened.
- {helper:Helper}: a named shortcut for a group of requirements (see [Helpers, tricks and setting rules](#helpers)).
- {trick:Trick}: only counts if the player turned that trick on.
- {setting:Setting}: only counts if a setting is chosen a certain way.
- {location:Check}: needs another check to be reachable.

Click a region, helper, trick, setting or event tag to go straight to it.
`,
    },
    {
      id: "editing",
      title: "Changing requirements",
      body: `
Click a row (an exit, check or event). The inspector on the right shows its requirements as **blocks**.

### Blocks

- An **All of** block needs everything in it.
- An **Any one of** block needs just one thing in it. These have a blue edge.
- Click the block's label to switch between "All of" and "Any one of".

Blocks can go inside blocks. "All of: {item:Bombs}, *Any one of*: {item:Hero's Bow}, {item:Hookshot}" is the example from the last section.

### Making changes

1. **Add requirement** opens a search box. Type part of a name (for example \`bow\`, \`clock town\`, \`goron\`), pick with the arrow keys or mouse, and press Enter.
2. To remove something, click the **×** on its tag.
3. **+ "any one of" group** (or **+ "all of" group**) adds a block inside the current one, for "this or that".
4. **x of** requirements, like "2 of these 4 items", have a number box you can change.

Every change appears on the page straight away, and Undo puts it back.

> If the block is empty it says **Nothing needed (always)**. That's correct for things that are free once you're in the region.

### Other things in the inspector

- **Leads to** (exits): where the exit goes. **Change** points it somewhere else.
- **Time of day** (checks, events, regions): see [Time of day](#time).
- **Used by** (events, helpers): every rule that needs this. Click one to go there.
- **Delete** removes the selected thing.

### Adding things

- **+ Add exit** on a region page connects it to another region. **New region…** creates a region and connects it in one go.
- **+ Add check** puts a check that isn't in any region yet into this one.
- **+ Add event** creates an event here. Give it a clear name like \`Drain the Well\`.
- **+ New region / helper / trick / setting rule** at the top of the navigator creates one from scratch.
`,
    },
    {
      id: "helpers",
      title: "Helpers, tricks and setting rules",
      body: `
### Helpers

A helper is a named group of requirements you'd otherwise repeat everywhere. For example, a helper for "can kill a Dexihand" might need any one of several weapons. Rules then just say {helper:Can Kill Dexihand}, and if you change the helper, every rule that uses it changes too.

Open helpers from the **Helpers** tab. The page shows what the helper requires and everything that **uses** it. Click its requirements to edit them in the inspector.

### Tricks

A trick is a technique a player might or might not want logic to expect, like a difficult jump or using a glitch. Players turn tricks on in the randomizer's settings, and only the tricks they turned on count.

On a trick's page you can edit:

- **What the trick is**: the description players see when choosing tricks. Make it clear.
- **Category**: the area it's grouped under in the tricks list.
- **Video or guide link**: where players can learn it. **Open** tests the link.

To use a trick, add it to a requirement like anything else. Usually it goes in an "any one of" block next to the normal way: *any one of {item:Hookshot}, {trick:Long Jump to Chest}*.

A trick can also require things itself (for example, a bomb jump needs {item:Bombs}). Those appear under **Also requires**.

### Setting rules

A setting rule counts only when a randomizer setting is chosen a certain way, like "Cows are closed" or "the boss key is randomized". On its page you choose the setting and the value it needs.

Some setting rules come built in. They're marked *"Built-in settings rule"*. They make logic follow settings that change what's possible, such as how many boss remains the Moon needs. You can read them to see exactly what each setting does to logic.
`,
    },
    {
      id: "time",
      title: "Time of day",
      body: `
Majora's Mask runs on a three-day cycle, so some things only happen at certain times. The editor splits the cycle into six parts: **D1 N1 D2 N2 D3 N3** (Day 1, Night 1, and so on). Day buttons light up gold and night buttons blue.

Select a check, event or region and look at **Time of day** in the inspector. There are three rows. If a row is empty it says **any**, meaning time doesn't matter for it.

- **Available**: when it can be done. A shop that's only open in the day would be D1 D2 D3. A Night 3 event would be N3.
- **Needed**: when having it matters. This is for events and helpers. For example, an event that only helps you on Night 3.
- **Setup**: when you have to prepare it, if that's different from when you collect it. For example, you might have to talk to someone on Day 1 to get a reward on Day 3.

The randomizer works out whether each check can be reached **at a time it's available**, taking the path there into account. If the way into a region is only open at night, the checks inside are only reachable at night too.

### Things that don't last

When you play the Song of Time, most things reset. The randomizer knows which items you keep (masks, songs, weapons) and which you lose (bombs you don't own a bag for, bottle contents, quest items like the Room Key). You don't need to mark these yourself.
`,
    },
    {
      id: "advanced",
      title: "Advanced time rules",
      body: `
Under **More time rules** in the inspector there are a few rarer options. Most checks never need them.

- **No way back this cycle.** After this, you can't keep playing the same cycle, so anything you find only helps after the Song of Time. The Moon and the Clock Tower roof work like this.
- **These times are opening hours.** The times under *Available* are when the place is **open**, so you have to be inside while it's open. Without this, being there at any of the times is enough.
- **Must happen before (same cycle).** List things that have to be done *earlier in the same cycle* than this, for example a reward that needs you to have started a quest on an earlier day. The built-in logic doesn't need this yet, but it's there for your own logic.
- **Also needs, if what's found here doesn't last.** Extra requirements that only apply when the item here would be lost on the Song of Time. The Bomb Shop's Old Lady rewards (like the {item:Blast Mask}) use this.

> If you're not sure whether you need one of these, you probably don't. The built-in logic already uses them where they matter.
`,
    },
    {
      id: "walkthroughs",
      title: "Walkthroughs",
      body: `
### Make a check need something more

Say you want logic to expect the {item:Lens of Truth} before a certain chest.

1. Search for the chest in the navigator and click its region.
2. Click the chest's row in **Checks**.
3. In the inspector, click **Add requirement**, type \`lens\` and press Enter.
4. The row now shows **needs** {item:Lens of Truth}. Save.

### Add a trick

1. Open the **Tricks** tab and click **+ New trick**. Give it a short name, like \`Clock Town Ledge Jump\`.
2. Fill in **What the trick is** (one or two sentences) and, if you have one, a video link.
3. Find the check or exit the trick helps with and click it.
4. Its normal requirement is, say, {item:Hookshot}. Add an "any one of" group, then put the {item:Hookshot} and your new {trick:Clock Town Ledge Jump} in it. Remove the old {item:Hookshot} from outside the group.
5. Now the check needs *either* the hookshot *or* the trick.

### Add a new area

1. Open the region the area is reached from.
2. Under **Ways out**, click **+ Add exit**, then **New region…**, and name it, for example \`Woodfall: Hidden Grotto\`.
3. The new region opens. Go back and set the exit's requirements (what you need to get in).
4. In the new region, use **+ Add check** to move checks into it, and **+ Add exit** to connect it back out if you can leave another way.

### Make your own shared helper

If you notice the same "any one of" list in lots of places, make it a helper:

1. **Helpers** tab, **+ New helper**, name it (for example \`Can Cross Gap\`).
2. Click the helper's requirements and add the alternatives.
3. In each rule that repeated the list, replace it with the helper.
`,
    },
    {
      id: "saving",
      title: "Problems, saving and using your logic",
      body: `
### Problems

The editor checks your logic as you go. The **Problems** button shows how many it found:

- **Errors** (red) are things that will break seeds, like an exit to a region that doesn't exist, a requirement that names something unknown, or something that needs itself.
- **Warnings** are things that are probably mistakes, like a region nothing leads to, the same requirement listed twice, or a trick with no description.

Click a problem to jump to it. Problems also show at the top of a region page and as red numbers in the navigator.

### Saving

- **Save** writes to the file you opened. The first time (or after opening built-in logic) it asks where to save.
- **Save as…** saves a copy somewhere new.
- **Export for older MMR versions…** saves in the old format so an older randomizer can read it. The map is flattened back into a plain list, and the advanced time rules are left out, because the old format has no way to store them.

### Using your logic in a seed

In the randomizer's settings, set **Logic** to custom logic and choose your file under **Custom logic file**. Seeds then use your logic.

### Tips

- Search finds checks too. Typing a check's name (or its id) shows the region it's in.
- Hover a tag to see its exact id.
- Keep notes. Helpers and tricks have a **Note** box for other logic writers. The randomizer ignores it.
- Change one thing at a time and keep an eye on Problems.
- Share your file. It's a plain text file anyone can open in the editor.

### Words used in this guide

| Word | Meaning |
| Region | A place in the game. |
| Exit | A one-way connection from one region to another, with its own requirements. |
| Check | A place an item can be found. |
| Event | Something that happens in a region that other rules can need. |
| Helper | A named group of requirements used in many places. |
| Trick | A technique logic only expects if the player turned it on. |
| Setting rule | A requirement that depends on a randomizer setting. |
| Cycle | One trip through the three days, ended by the Song of Time. |
`,
    },
  ],
};

const veteran: Guide = {
  id: "veteran",
  title: "Coming from the old editor",
  blurb: "You know RequiredItems, ConditionalItems and the Area* items. Here's what changed.",
  sections: [
    {
      id: "summary",
      title: "What changed, in one page",
      body: `
If you wrote logic in the old MMR editor, nothing you know is wrong. The ideas are the same, but they're organised differently:

- **The old list is now a map.** The old logic was one long list where every item and location had *Required items* and *Conditional items*. The \`Area…\` and \`… Access\` entries quietly acted as places. Now those places are real **regions** connected by **exits**, and every check sits inside one.
- **Rules are local.** A check's rule only says what it needs *once you're in its region*. The access requirements live on the exits, once, instead of being repeated in every location.
- **Settings code is now visible logic.** The old randomizer changed logic in C# depending on settings (boss remains, progressive upgrades, and so on). Those changes are now ordinary rules you can read, marked *Built-in settings rule*.
- **Setting expressions are structured.** No more C# strings. A setting rule picks a setting and a value.
- **Time has a few new options**, for things the old model couldn't express. They're all optional.
- **Old files still work.** Open an old logic file and it's converted to regions, then **checked to behave identically**. You can also export back to the old format.

The rest of this guide goes through each of these in detail.
`,
    },
    {
      id: "mapping",
      title: "Old term, new term",
      body: `
| Old editor | New editor |
| Item / location entry | A **check** inside a region (on the region page under *Checks*) |
| \`Area…\` / \`… Access\` fake item | A **region**, reached by exits |
| Other fake items (\`Can Use…\`, \`Kill…\`) | A **helper** (Helpers tab) |
| Fake items for one-off happenings (boss defeated, water drained) | An **event** inside a region |
| Required items | The **needs** line, or the top **All of** block |
| Conditional items (each set) | The numbered lines under **and one of**, or an **Any one of** block |
| Is trick + tooltip / category / URL | A **trick** with *What the trick is*, *Category* and *Video or guide link* |
| Setting expression (C#) | A **setting rule** with a structured condition |
| Time needed / available / setup | **Needed / Available / Setup** under *Time of day*: same meaning |
| "Used by" search | **Used by** on helper, event and trick pages and in the inspector |
| Settings logic in code (\`UpdateLogicForSettings\`) | Visible helpers marked *Built-in settings rule* |
| Logic file version | Detected automatically. Old files are converted on open. |

### Ids

Entries are still identified by their old ids (for example \`ItemBow\` or \`HeartPieceNotebookMayor\`), so anything you remember still finds the right thing. The editor shows readable names; hover a tag to see its id, and search matches ids too.

Regions are referred to as \`Region: <name>\` inside rules.
`,
    },
    {
      id: "rules",
      title: "Required and conditional items, now",
      body: `
The old pair *Required items* + *Conditional items* meant: **all of these, and at least one of these sets**. The new rules can say the same thing, plus anything else, by nesting:

- **All of** = every child needed (your old Required items).
- **Any one of** = one child needed (your old list of Conditional sets).
- Groups can be nested any number of levels deep, so you no longer need a fake item just to put an "or" inside an "and".
- **N of** (for example "2 of these 4") is built in.

### How rules are displayed

The page shows each rule in a standard order, which lines up with the old way of thinking:

- **needs** is the part every alternative shares: the old *Required items*.
- **and one of**, with numbered lines, are the alternatives: the old *Conditional items* sets, with the shared parts already taken out.
- Within a line, tags are sorted: regions, events, items, checks, helpers, settings, then tricks.

Because of this you'll often see a rule displayed more neatly than you typed it. The editor works out the shared part for you. Very large rules (more than 40 alternatives) fall back to a compact one-line form.

### Editing

Click the row and edit the blocks in the inspector. Rules on helpers are still stored in the old Required/Conditional shape, so they convert back cleanly, and the editor rearranges what you build to fit.
`,
    },
    {
      id: "regions",
      title: "Thinking in regions",
      body: `
### Where did my Area items go?

In old logic you'd write a location as, say, Required items: \`AreaWoodFallTempleAccess\`, \`MaskDeku\`. Here, \`AreaWoodFallTempleAccess\` is the region {region:Woodfall Temple}, and the location sits **inside** it with just *needs {item:Deku Mask}*.

When the converter turns old logic into regions it:

1. makes a region for each place entry, entered from its parent area by an exit carrying the place's rule;
2. moves each location into the deepest region whose places its rule needed, and keeps only the **rest** of the rule on the check;
3. leaves locations that didn't name a place in the region they're physically in, with their full rule.

### What this changes in practice

- **Fewer repeats.** You change temple access once, on the exit, instead of in every chest.
- **Ways in and Ways out** on each page show you how a place is connected, which the old list never could.
- **Multiple ways in** are just multiple exits. You don't need one big "or" in an Access item.
- **A region is a requirement too.** You can still require a place from anywhere, for example *needs {region:Ikana Canyon}*, exactly like requiring an \`Area\` item.

### Start region

One region is marked **Start**. That's where reachability begins (the old logic had no explicit start). Change it from **Region settings → Make this the starting region**.
`,
    },
    {
      id: "settings",
      title: "Settings in logic",
      body: `
### Setting expressions are now structured

Old setting expressions were C# compiled at run time, like \`settings.DamageMode == DamageMode.Default\`. Now a setting rule is data: choose the setting, then the value. The options are:

- a yes/no setting is **on** (for example "Close cows");
- a choice setting **is** a value (for example damage mode is *Default*);
- a multi-choice setting **has** a value (for example enemy mode includes *Randomized*);
- a location **is randomized**;
- **not** any of the above.

Old expressions are translated when you import. They're the same checks, so the behaviour doesn't change.

### The randomizer's hidden logic changes are now visible

The old randomizer rewrote logic in code before every seed, depending on settings. For example:

- how many boss remains the Moon needs;
- victory conditions (all fairies, all masks, all notebook entries…);
- progressive upgrades (sword, wallet, bomb bag, quiver, magic, Lullaby), where finding upgrades in any order counts;
- *Bring your own ammo*, hint masks, and a few others.

None of that showed up in the editor, so the logic you edited wasn't quite the logic that ran. Now each rewrite is written into the logic as ordinary helpers that depend on setting rules, with the note *"Built-in settings rule (older MMR versions applied this in code)."*

So what you see is what runs. You can open those helpers to see exactly what a setting does, and edit them if you want different behaviour. Tests check that they match what the old code did under every setting.

When a seed is made, rules for settings that aren't chosen drop out before anything else happens, so they cost nothing.
`,
    },
    {
      id: "time",
      title: "Time: what's the same and what's new",
      body: `
### Same as before

**Needed**, **Available** and **Setup** mean exactly what they did, over the same six periods (D1 N1 D2 N2 D3 N3). An empty row means "any time". The randomizer uses the same model as the old one: a location is reachable if its path and its own times overlap.

Regions can have times as well, which applies the times to everything inside.

### Temporary items are automatic

The old logic needed care so that things lost on the Song of Time (bottle contents, Kafei's quest items, ammo without the upgrade) weren't treated as kept. MMR Next knows which items reset and tracks them per cycle, so you don't need workarounds for this.

### New, optional time rules

These are under **More time rules** in the inspector. They describe things the old format had no way to say, and the built-in logic uses them where the old code had special cases:

- **No way back this cycle** (\`no_return\`). Nothing found here helps until after the Song of Time. Used for the Moon and the Clock Tower roof.
- **These times are opening hours** (\`opening_hours\`). The *Available* times are when the place is open, so you have to be inside while it's open. Used for shops and houses, like the Curiosity Shop and the Mayor's Residence.
- **Must happen before (same cycle)** (\`before\`). Other things that must be done earlier in the same cycle. Available for your own logic; the built-in logic doesn't need it yet.
- **Also needs, if what's found here doesn't last** (\`temporary_requires\`). Extra requirements that only apply when the reward here would be lost on the Song of Time. Used for the Bomb Shop Old Lady's rewards.

None of these exist in the old format. **Export for older MMR versions** leaves them out.
`,
    },
    {
      id: "files",
      title: "Opening old files and exporting",
      body: `
### Importing

**Open → Open a file…** accepts any MMR logic file. When it's in the old single-list format, the editor:

1. reads it and builds the settings rewrites into it (see [Settings in logic](#settings));
2. converts it into regions;
3. **checks that the result behaves exactly like the original**, by testing every rule in hundreds of different situations (different items owned, tricks on and off, settings, times of day).

A banner at the top tells you how it went. Green means every rule matched. If anything differs, the banner names the rules to look at before saving.

Very old files (from before the logic JSON format) need opening and saving once in a newer version of the old randomizer first; the editor tells you if so.

The first save after importing asks for a new file name, so the original is never overwritten.

### Exporting

**Save → Export for older MMR versions…** writes the old format:

- regions are flattened back into the old \`Region: …\` entries and location rules;
- setting rules are written back as C# setting expressions;
- tricks keep their tooltip, category and URL;
- the new time rules are left out.

The built-in settings rules come along as ordinary entries. The old randomizer still applies its own code changes on top of them, so if you rely on unusual settings, test a few seeds there.
`,
    },
    {
      id: "workflow",
      title: "Doing your usual jobs",
      body: `
### "Find what a location needs"

Search for it in the navigator (name or old id). The regions list narrows to the one it's in; open it and the check is under **Checks**, with its own rule on the row. To see what it takes to *get there*, read **Ways in**, and click one to go a step back.

### "Find everything that uses X"

Open X (a helper, trick, setting rule or event) and read **Used by**. Every entry is a link to the rule that uses it.

### "Add a trick"

**Tricks** tab → **+ New trick**. Fill in the description (the old tooltip), category and link. Then add the trick to the rule it unlocks, usually in an **Any one of** block next to the normal requirement.

### "Change access to a whole area"

Edit the exit into that area's region, not each location. Every check inside follows automatically.

### "Make a new fake item"

Decide what it is:

- a **place** becomes a region (*+ Add exit → New region…* on the page it's reached from);
- a **reusable group of requirements** becomes a helper (*+ New helper*);
- **something that happens** becomes an event (*+ Add event* in the region where it happens).

### Validation

The **Problems** list covers what the old editor's checks did, and more: unknown references, things that need themselves (including loops through other entries), duplicate or redundant requirements, empty groups, tricks without descriptions, exits to missing regions and regions nothing leads to.
`,
    },
  ],
};

export const GUIDES: Guide[] = [beginner, veteran];
