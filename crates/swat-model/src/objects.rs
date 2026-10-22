//! Spatial objects and where their water goes: the `*.con` files (`hyd_read_connect`).

/// The kinds of spatial object, one per connection file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ObjectKind {
    Hru,
    HruLte,
    RoutingUnit,
    Gwflow,
    Aquifer,
    Aquifer2d,
    Channel,
    Reservoir,
    Recall,
    ExportCoefficient,
    DeliveryRatio,
    Outlet,
    /// SWAT-DEG channel (`chandeg.con`), the channel SWAT+ routes today.
    ChannelDeg,
}

impl ObjectKind {
    /// Every kind, in the order of the `connect` row of `file.cio`.
    pub const ALL: [ObjectKind; 13] = [
        ObjectKind::Hru,
        ObjectKind::HruLte,
        ObjectKind::RoutingUnit,
        ObjectKind::Gwflow,
        ObjectKind::Aquifer,
        ObjectKind::Aquifer2d,
        ObjectKind::Channel,
        ObjectKind::Reservoir,
        ObjectKind::Recall,
        ObjectKind::ExportCoefficient,
        ObjectKind::DeliveryRatio,
        ObjectKind::Outlet,
        ObjectKind::ChannelDeg,
    ];

    /// The slot of its connection file in the `connect` row of `file.cio`.
    pub fn cio_slot(self) -> &'static str {
        match self {
            ObjectKind::Hru => "hru_con",
            ObjectKind::HruLte => "hruez_con",
            ObjectKind::RoutingUnit => "ru_con",
            ObjectKind::Gwflow => "gwflow_con",
            ObjectKind::Aquifer => "aqu_con",
            ObjectKind::Aquifer2d => "aqu2d_con",
            ObjectKind::Channel => "chan_con",
            ObjectKind::Reservoir => "res_con",
            ObjectKind::Recall => "rec_con",
            ObjectKind::ExportCoefficient => "exco_con",
            ObjectKind::DeliveryRatio => "delr_con",
            ObjectKind::Outlet => "out_con",
            ObjectKind::ChannelDeg => "chandeg_con",
        }
    }

    /// The type code other objects use to send water to this kind (`obj_typ` in `*.con`, as
    /// `hyd_connect` reads it); `None` for kinds that receive no water.
    pub fn code(self) -> Option<&'static str> {
        Some(match self {
            ObjectKind::Hru => "hru",
            ObjectKind::HruLte => "hlt",
            ObjectKind::RoutingUnit => "ru",
            ObjectKind::Gwflow => "gwflow",
            ObjectKind::Aquifer => "aqu",
            ObjectKind::Channel => "cha",
            ObjectKind::Reservoir => "res",
            ObjectKind::ExportCoefficient => "exc",
            ObjectKind::DeliveryRatio => "dr",
            ObjectKind::Outlet => "out",
            ObjectKind::ChannelDeg => "sdc",
            ObjectKind::Aquifer2d | ObjectKind::Recall => return None,
        })
    }
}

/// A spatial object: one row of a connection file.
#[derive(Clone, Debug, PartialEq)]
pub struct SpatialObject {
    pub kind: ObjectKind,
    /// Number within its kind (`id`).
    pub id: i32,
    pub name: String,
    pub gis_id: i32,
    pub area_ha: f32,
    pub lat: f32,
    pub lon: f32,
    pub elev: f32,
    /// Its row in the kind's data file (`hru`, `rtu`, `aqu`, `lcha`... column; SWAT+ `props`).
    pub props: i32,
    /// Weather station name (`wst`).
    pub weather_station: String,
    /// Constituent data pointer (`cst`).
    pub constit: i32,
    /// Overbank or other second pointer (`ovfl`; SWAT+ `props2`).
    pub props2: i32,
    /// Flow rule set (`rule`).
    pub ruleset: String,
    pub outflows: Vec<Outflow>,
}

/// Where a share of an object's water goes.
#[derive(Clone, Debug, PartialEq)]
pub struct Outflow {
    /// Receiving object's type code (`obj_typ`, see [`ObjectKind::code`]).
    pub obj_type: String,
    /// Receiving object's number within its kind (`obj_id`).
    pub obj_id: i32,
    /// Which part of the water: `tot`, `sur`, `lat`, `til`, `rhg`, `aqu`... (`hyd_typ`).
    pub hyd_type: String,
    /// Fraction of that part sent (`frac`).
    pub frac: f32,
}
