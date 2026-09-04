use serde::{Deserialize, Serialize};

/// 影子节点的方向。
/// Inflow：入向影子，普通节点的影子，产生边的源端不是画布节点，在画布内只能有出度（只能作为源）；
/// Outflow：出向影子，画布节点的影子，产生边的源端是画布节点，在画布内只能有入度（只能作为目标）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ShadowDirection {
    Inflow,
    Outflow,
}
