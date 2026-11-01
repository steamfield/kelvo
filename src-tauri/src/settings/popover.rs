//! Partial presentation settings, independent of collector configuration.
use super::set;
use serde::Deserialize;
use statglass_schema::settings::PopoverSettings;

#[derive(Clone, Copy, Debug, Default, PartialEq, Deserialize, specta::Type)]
#[serde(default)]
pub struct PopoverPatch {
    #[specta(optional)]
    pub cpu: Option<statglass_schema::settings::CardDensity>,
    #[specta(optional)]
    pub memory: Option<statglass_schema::settings::CardDensity>,
    #[specta(optional)]
    pub gpu: Option<statglass_schema::settings::CardDensity>,
    #[specta(optional)]
    pub power: Option<statglass_schema::settings::CardDensity>,
    #[specta(optional)]
    pub network: Option<statglass_schema::settings::CardDensity>,
    #[specta(optional)]
    pub battery: Option<statglass_schema::settings::CardDensity>,
    #[specta(optional)]
    pub containers: Option<statglass_schema::settings::CardDensity>,
}

impl PopoverPatch {
    pub fn apply(self, target: &mut PopoverSettings) {
        set(&mut target.cpu, self.cpu);
        set(&mut target.memory, self.memory);
        set(&mut target.gpu, self.gpu);
        set(&mut target.power, self.power);
        set(&mut target.network, self.network);
        set(&mut target.battery, self.battery);
        set(&mut target.containers, self.containers);
    }
}
