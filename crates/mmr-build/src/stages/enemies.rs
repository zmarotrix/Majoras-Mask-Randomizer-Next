//! Enemy randomization (`WriteEnemies` with `Enemies.ShuffleEnemies`): each
//! scene's enemies are swapped for others of the same kind (ground, air,
//! water...), keeping object memory no bigger than before.

use crate::dotnet_random::DotNetRandom;
use crate::enums;
use crate::scene::{self, Scene};
use crate::{BuildError, Builder};

const OBJECT_TABLE: u32 = 0xC5_8C80;
/// Scenes left alone.
const SKIP_SCENES: [u8; 5] = [0x08, 0x20, 0x24, 0x4F, 0x69];

/// An enemy the shuffle knows (`Models.Rom.Enemy`).
#[derive(Debug, Clone, PartialEq)]
struct Enemy {
    actor: u16,
    object: u16,
    object_size: i32,
    variables: Vec<u16>,
    kind: String,
    stationary: bool,
    forbid_from: Vec<u8>,
    forbid_to: Vec<u8>,
}

/// .NET's `list.Random(rng)`.
fn pick<'a, T>(list: &'a [T], rng: &mut DotNetRandom) -> &'a T {
    if list.len() == 1 {
        &list[0]
    } else {
        &list[rng.next_below(list.len() as i32) as usize]
    }
}

impl Builder<'_> {
    pub(crate) fn write_enemies(&mut self) -> Result<(), BuildError> {
        if !self.plan.has("EnemyMode", "Randomized") {
            return Ok(());
        }
        let mut rng = DotNetRandom::new(self.plan.seed as i32);
        let enemies = self.read_enemy_list()?;
        // A fresh read of every scene, with alternate room headers and actors.
        let mut scenes = scene::read_scenes(&mut self.rom)?;
        scene::read_map_actors(&mut self.rom, &mut scenes)?;
        for s in scenes.iter_mut().filter(|s| !SKIP_SCENES.contains(&s.number)) {
            if swap_scene_enemies(s, &enemies, &mut rng)? {
                scene::write_maps(&mut self.rom, s)?;
            }
        }
        self.scenes = scenes;
        Ok(())
    }

    /// `Enemies.ReadEnemyList`.
    fn read_enemy_list(&mut self) -> Result<Vec<Enemy>, BuildError> {
        let scene_ids = enums::get("Scene");
        let scene_id = |name: &str| scene_ids.attr(name, "SceneInternalId").and_then(|a| a.get("InternalId").as_u64()).unwrap_or(0) as u8;
        let f = self.rom.index_of(OBJECT_TABLE).unwrap();
        self.rom.open(f)?;
        let base = (OBJECT_TABLE - self.rom.files[f].addr) as usize;
        let table = self.rom.files[f].data.clone();
        let mut out = Vec::new();
        for v in &enums::get("Enemy").values {
            let attr = |k: &str| v.attributes.iter().find(|a| a.kind == k);
            let all = |k: &'static str| v.attributes.iter().filter(move |a| a.kind == k);
            let id = |k: &str| attr(k).and_then(|a| a.get("Id").as_u64()).unwrap_or(0) as u16;
            let object = id("ObjectId");
            let at = base + object as usize * 8;
            let size = crate::be32(&table, at + 4).wrapping_sub(crate::be32(&table, at)) as i32;
            out.push(Enemy {
                actor: id("ActorId"),
                object,
                object_size: size,
                variables: all("Variable").filter_map(|a| a.get("Variable").as_u64()).map(|x| x as u16).collect(),
                kind: attr("ActorType").and_then(|a| a.get("Type").as_str()).unwrap_or("").to_string(),
                stationary: attr("IsMoving").is_none(),
                forbid_from: all("ForbidFromScene").filter_map(|a| a.get("Scene").as_str()).map(scene_id).collect(),
                forbid_to: all("ForbidToScene").filter_map(|a| a.get("Scene").as_str()).map(scene_id).collect(),
            });
        }
        Ok(out)
    }
}

/// Enemies that could replace these, by kind (`GetMatchPool`). Movement
/// usually has to match too, but one time in five it needn't.
fn match_pool(scene_enemies: &[&Enemy], allowed: &[&Enemy], rng: &mut DotNetRandom) -> Vec<Enemy> {
    let mut pool: Vec<Enemy> = Vec::new();
    for s in scene_enemies {
        for e in allowed {
            let fits = e.kind == s.kind && (e.stationary == s.stationary || rng.next_below(5) == 0);
            if fits && !pool.contains(e) {
                pool.push((*e).clone());
            }
        }
    }
    pool
}

/// `Enemies.SwapSceneEnemies`. Returns whether the scene should be written
/// back.
fn swap_scene_enemies(scene: &mut Scene, list: &[Enemy], rng: &mut DotNetRandom) -> Result<bool, BuildError> {
    let known = |actor: u16| list.iter().find(|e| e.actor == actor);
    let mut actors: Vec<u16> = Vec::new();
    for map in &scene.maps {
        for a in &map.actors {
            if let Some(e) = known(a.n) {
                if !e.forbid_from.contains(&scene.number) {
                    actors.push(e.actor);
                }
            }
        }
    }
    if actors.is_empty() {
        return Ok(false);
    }
    let mut objects: Vec<u16> = Vec::new();
    for map in &scene.maps {
        for &o in &map.objects {
            if let Some(e) = list.iter().find(|e| e.object == o) {
                if !objects.contains(&e.object) && !e.forbid_from.contains(&scene.number) {
                    objects.push(e.object);
                }
            }
        }
    }
    if objects.is_empty() {
        return Ok(false);
    }
    // Objects whose enemies aren't placed here are loaded for something else.
    objects.retain(|&o| list.iter().filter(|e| e.object == o).any(|e| actors.contains(&e.actor)));

    let allowed: Vec<&Enemy> = list.iter().filter(|e| !e.forbid_to.contains(&scene.number)).collect();
    let (obj_updates, updates, matches) = loop {
        let mut obj_updates: Vec<(u16, u16)> = Vec::new();
        let mut updates: Vec<Vec<&Enemy>> = Vec::new();
        let mut matches: Vec<Vec<Enemy>> = Vec::new();
        let (mut old_size, mut new_size) = (0i64, 0i64);
        for &o in &objects {
            let scene_enemies: Vec<&Enemy> = list.iter().filter(|e| e.object == o && actors.contains(&e.actor)).collect();
            let pool = match_pool(&scene_enemies, &allowed, rng);
            if pool.is_empty() {
                return Err(BuildError::Data(format!("enemizer: nothing can replace object {o:#X} in scene {}", scene.number)));
            }
            let choice = pick(&pool, rng).clone();
            new_size += choice.object_size as i64;
            old_size += scene_enemies[0].object_size as i64;
            obj_updates.push((o, choice.object));
            updates.push(scene_enemies);
            matches.push(pool);
        }
        // Rough, but keeps scenes from running out of object memory.
        if new_size <= old_size {
            break (obj_updates, updates, matches);
        }
    };

    let mut actor_updates: Vec<(u16, u16, u16)> = Vec::new();
    for (i, &(_, new_object)) in obj_updates.iter().enumerate() {
        let mut j = 0;
        while j != actors.len() {
            let actor = actors[j];
            let Some(old) = updates[i].iter().find(|e| e.actor == actor) else {
                j += 1;
                continue;
            };
            let subs: Vec<&Enemy> = matches[i].iter().filter(|e| e.object == new_object).collect();
            let enemy = loop {
                let e = *pick(&subs, rng);
                if old.kind == e.kind && old.stationary == e.stationary {
                    break e;
                }
                if old.kind == e.kind && rng.next_below(5) == 0 {
                    break e;
                }
                if !subs.iter().any(|s| s.kind == old.kind) {
                    break e;
                }
            };
            let variable = *pick(&enemy.variables, rng);
            actor_updates.push((actor, enemy.actor, variable));
            actors.remove(j);
        }
    }
    // `SetSceneEnemyActors`, then `SetSceneEnemyObjects`.
    for map in scene.maps.iter_mut() {
        for a in map.actors.iter_mut() {
            if let Some(k) = actor_updates.iter().position(|u| u.0 == a.n) {
                let (_, n, v) = actor_updates.remove(k);
                a.n = n;
                a.variable = v;
            }
        }
    }
    for map in scene.maps.iter_mut() {
        for o in map.objects.iter_mut() {
            if let Some(&(_, new)) = obj_updates.iter().find(|u| u.0 == *o) {
                *o = new;
            }
        }
    }
    Ok(true)
}
