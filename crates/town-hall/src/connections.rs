use anyhow::{Result, ensure};
use neuro_ash::DenseNetworkPool;

use crate::{
    CONNECTION_BUNDLE_SIZE, INPUT_HEAD_SIZE, TARGET_HEAD_SIZE, give_output, predict_output,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConnectionBundle {
    pub input_connections: [f32; INPUT_HEAD_SIZE],
    pub predict_connections: [f32; TARGET_HEAD_SIZE - 1],
    pub predict_bias: f32,
    pub give_connections: [f32; TARGET_HEAD_SIZE - 1],
    pub give_bias: f32,
}

impl ConnectionBundle {
    pub fn from_output(output: &[f32]) -> Result<Self> {
        ensure!(
            output.len() == CONNECTION_BUNDLE_SIZE,
            "connection output has {} values, expected {CONNECTION_BUNDLE_SIZE}",
            output.len()
        );
        ensure!(
            output.iter().all(|value| value.is_finite()),
            "get-to-know network produced a non-finite connection"
        );

        let mut cursor = 0;
        let input_connections = output[cursor..cursor + INPUT_HEAD_SIZE].try_into().unwrap();
        cursor += INPUT_HEAD_SIZE;
        let predict_connections = output[cursor..cursor + TARGET_HEAD_SIZE - 1]
            .try_into()
            .unwrap();
        cursor += TARGET_HEAD_SIZE - 1;
        let predict_bias = output[cursor];
        cursor += 1;
        let give_connections = output[cursor..cursor + TARGET_HEAD_SIZE - 1]
            .try_into()
            .unwrap();
        cursor += TARGET_HEAD_SIZE - 1;
        let give_bias = output[cursor];

        Ok(Self {
            input_connections,
            predict_connections,
            predict_bias,
            give_connections,
            give_bias,
        })
    }

    pub fn install(
        &self,
        action_brains: &mut DenseNetworkPool,
        observer: usize,
        slot: usize,
    ) -> Result<()> {
        action_brains.set_input_connections(
            observer,
            crate::BASE_INPUT_COUNT + slot,
            &self.input_connections,
        )?;
        action_brains.set_output_neuron(
            observer,
            predict_output(slot),
            &self.predict_connections,
            self.predict_bias,
        )?;
        action_brains.set_output_neuron(
            observer,
            give_output(slot),
            &self.give_connections,
            self.give_bias,
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_is_split_at_the_three_head_boundaries() {
        let output = (0..CONNECTION_BUNDLE_SIZE)
            .map(|index| index as f32)
            .collect::<Vec<_>>();

        let bundle = ConnectionBundle::from_output(&output).unwrap();

        assert_eq!(bundle.input_connections[0], 0.0);
        assert_eq!(
            bundle.input_connections[INPUT_HEAD_SIZE - 1],
            (INPUT_HEAD_SIZE - 1) as f32
        );
        assert_eq!(bundle.predict_connections[0], INPUT_HEAD_SIZE as f32);
        assert_eq!(
            bundle.predict_bias,
            (INPUT_HEAD_SIZE + TARGET_HEAD_SIZE - 1) as f32
        );
        assert_eq!(
            bundle.give_connections[0],
            (INPUT_HEAD_SIZE + TARGET_HEAD_SIZE) as f32
        );
        assert_eq!(bundle.give_bias, (CONNECTION_BUNDLE_SIZE - 1) as f32);
    }

    #[test]
    fn non_finite_generated_connections_are_rejected() {
        let mut output = vec![0.0; CONNECTION_BUNDLE_SIZE];
        output[INPUT_HEAD_SIZE] = f32::NAN;

        let error = ConnectionBundle::from_output(&output).unwrap_err();

        assert!(error.to_string().contains("non-finite"));
    }
}
