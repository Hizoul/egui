use ahash::AHashMap;

use crate::{math::Rect, paint::PaintCmd, Id};

/// Different layer categories
#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum Order {
    /// Painted behind all floating windows
    Background,
    /// Normal moveable windows that you reorder by click
    Middle,
    /// Popups, menus etc that should always be painted on top of windows
    Foreground,
    /// Foreground objects can also have tooltips
    Tooltip,
    /// Debug layer, always painted last / on top
    Debug,
}
impl Order {
    const COUNT: usize = 5;
    const ALL: [Order; Self::COUNT] = [
        Self::Background,
        Self::Middle,
        Self::Foreground,
        Self::Tooltip,
        Self::Debug,
    ];
}

/// An identifier for a paint layer.
/// Also acts as an identifier for `Area`:s.
#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct LayerId {
    pub order: Order,
    pub id: Id,
}

impl LayerId {
    pub fn debug() -> Self {
        Self {
            order: Order::Debug,
            id: Id::new("debug"),
        }
    }

    pub fn background() -> Self {
        Self {
            order: Order::Background,
            id: Id::background(),
        }
    }
}

/// A unique identifier of a specific `PaintCmd` in a `PaintList`.
#[derive(Clone, Copy, PartialEq)]
pub struct PaintCmdIdx(usize);

/// Each `PaintCmd` is paired with a clip rectangle.
#[derive(Clone, Default)]
pub struct PaintList(Vec<(Rect, PaintCmd)>);

impl PaintList {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the index of the new command that can be used with `PaintList::set`.
    pub fn add(&mut self, clip_rect: Rect, cmd: PaintCmd) -> PaintCmdIdx {
        let idx = PaintCmdIdx(self.0.len());
        self.0.push((clip_rect, cmd));
        idx
    }

    pub fn extend(&mut self, clip_rect: Rect, mut cmds: Vec<PaintCmd>) {
        self.0.extend(cmds.drain(..).map(|cmd| (clip_rect, cmd)))
    }

    /// Modify an existing command.
    ///
    /// Sometimes you want to paint a frame behind some contents, but don't know how large the frame needs to be
    /// until the contents have been added, and therefor also painted to the `PaintList`.
    ///
    /// The solution is to allocate a `PaintCmd` using `let idx = paint_list.add(cr, PaintCmd::Noop);`
    /// and then later setting it using `paint_list.set(idx, cr, frame);`.
    pub fn set(&mut self, idx: PaintCmdIdx, clip_rect: Rect, cmd: PaintCmd) {
        assert!(idx.0 < self.0.len());
        self.0[idx.0] = (clip_rect, cmd);
    }
}

#[derive(Clone, Default)]
pub struct GraphicLayers([AHashMap<Id, PaintList>; Order::COUNT]);

impl GraphicLayers {
    pub fn list(&mut self, layer_id: LayerId) -> &mut PaintList {
        self.0[layer_id.order as usize]
            .entry(layer_id.id)
            .or_default()
    }

    pub fn drain(
        &mut self,
        area_order: &[LayerId],
    ) -> impl ExactSizeIterator<Item = (Rect, PaintCmd)> {
        let mut all_commands: Vec<_> = Default::default();

        for &order in &Order::ALL {
            let order_map = &mut self.0[order as usize];

            // If a layer is empty at the start of the frame
            // the nobody has added to it, and it is old and defunct.
            // Free it to save memory:
            order_map.retain(|_, list| !list.is_empty());

            // First do the layers part of area_order:
            for layer_id in area_order {
                if layer_id.order == order {
                    if let Some(commands) = order_map.get_mut(&layer_id.id) {
                        all_commands.extend(commands.0.drain(..));
                    }
                }
            }

            // Also draw areas that are missing in `area_order`:
            for commands in order_map.values_mut() {
                all_commands.extend(commands.0.drain(..));
            }
        }

        all_commands.into_iter()
    }
}
