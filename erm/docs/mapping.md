# Component

```rs
struct Position {
    x,
    y
}

struct Velocty {
    x,
    y
}

struct PhysicalObject {
    position: Position,
    velocity: Velocity,
}
```

Simple case `Vec<PhysicalObject>`
```sql
with
    position as (
        select
            entity, x, y
        from
            erm_position
    ),
    velocity as (
        select
            entity, x, y
        from
            erm_velocity
    ),
    compound as (
        select
            position.entity,
            position.x,
            position.y,
            velocity.x,
            velocity.y
        from position
        inner join velocity
        on position.entity == velocity.entity
    )
select * from compound
```

Singular Case `PhysicalObject`
```sql
-- As above
where compound.entity == $1
```

# Referenced

```rs
struct Parent {
    parent: Entity
}

struct LabeledNode {
    name: String,
}

struct Link {
    parent: Parent,
    label: LabeledNode,
}
```

```sql
with
    parent as (
        select
            entity,
            parent
        from
            parents
    ),
    nodes as (
        select
            entity,
            label
        from labeled_nodes
    ),
    compound as (
        select
            parents.parent,
            nodes.label
        from parents
        inner join nodes
        on parents.entity == nodes.entity
    )
select *
from compound