# Patrones de ejemplo testeable headless (sin GPU)

El workspace del libro usa `bevy = { version = "=0.19.0", default-features = false }` para que los tests corran en CI sin GPU. Esto fuerza un estilo de ejemplo concreto.

## Patrón: lógica pura sobre `World`
Construye entidades y sistemas sobre un `World` directo, sin `App::new().run()`:

```rust
use bevy::prelude::*;
use bevy::ecs::hierarchy::ChildOf;

#[test]
fn hierarchy_builds() {
    let mut world = World::new();
    let parent = world.spawn(Name::new("Village")).id();
    let child = world.spawn((Name::new("House"), ChildOf(parent))).id();

    assert!(world.get::<bevy::ecs::hierarchy::Children>(parent).is_some());
}
```

## Patrón: concepto subyacente en lugar de API ergonómica
Cuando una API (como `bsn!`) es difícil de testear sin render, enseña el **concepto subyacente** que automatiza:

- `bsn!` → enseñar `ChildOf`/`Children`/`#[require]` (lo que la macro genera).
- `Material` → enseñar la lógica del componente sin el shader GPU.
- `AudioSource` → enseñar el evento de audio sin reproducir.

La `code-pedagogy-justifier` declara este `tradeoff` en la code card.

## Patrón: sistema aislado
Extrae la lógica del sistema en una función pura testeable:

```rust
fn apply_damage(health: &mut Health, amount: i32) { health.current -= amount; }

#[test]
fn damage_reduces_health() {
    let mut h = Health { current: 100, max: 100 };
    apply_damage(&mut h, 30);
    assert_eq!(h.current, 70);
}
```

## Lo que NO hacer
- `App::new().add_plugins(DefaultPlugins).run()` en un test → cuelga en CI sin GPU.
- Depender de `bevy_render`/`bevy_sprite` con features que requieren GPU.
- Dejar un crate sin `#[cfg(test)]` → no aporta cobertura.

## Crates stub (capítulos GPU)
Para rendering/shaders/audio/UI/networking: un crate que **compila** pero con tests mínimos o ninguno. Documentar en el README del crate que requiere GPU para tests funcionales (como hace chapter-14+ del repo real).
