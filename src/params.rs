use nih_plug::prelude::*;

#[derive(Params)]
pub struct MidiTransposerParams {
    /// The parameter's ID is used to identify the parameter in the wrappred plugin API. As long as
    /// these IDs remain constant, you can rename and reorder these fields as you wish. The
    /// parameters are exposed to the host in the same order they were defined.
    #[id = "in_channel"]
    pub in_channel: IntParam,
    #[id = "out_channel"]
    pub out_channel: IntParam
}

impl Default for MidiTransposerParams {
    fn default() -> Self {
        Self {
            in_channel: IntParam::new(
                "Input Channel",
                1,
                IntRange::Linear { min: 0, max: 16 }
            ),
            out_channel: IntParam::new(
                "Output Channel",
                1,
                IntRange::Linear { min: 0, max: 16 }
            )
            
        }
    }
}