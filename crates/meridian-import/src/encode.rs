use crate::{
    ImportError, Result,
    palette::LINEAR_BITS,
    source::{Design, Port},
};

fn finite_position(position: &[f64; 3]) -> bool {
    position
        .iter()
        .all(|value| value.is_finite() && value.abs() <= 1e12)
}

impl Design {
    pub(crate) fn validate(&self) -> Result<()> {
        if self.license != "MIT" || self.appearance_license != "MIT" {
            return Err(ImportError::Identity(
                "Selected design/appearance notices differ",
            ));
        }
        if self.vertices.is_empty()
            || self.faces.is_empty()
            || self.vertices.iter().any(|v| !finite_position(v))
        {
            return Err(ImportError::Geometry("Invalid finite 3D vertex data"));
        }
        if self
            .faces
            .iter()
            .flatten()
            .any(|&index| index as usize >= self.vertices.len())
        {
            return Err(ImportError::Geometry("Triangle index is out of range"));
        }
        if self.faces.len() != self.face_materials.len() {
            return Err(ImportError::Geometry(
                "Every triangle needs its original material assignment",
            ));
        }
        if self
            .face_materials
            .iter()
            .any(|&index| index as usize >= self.materials.len())
        {
            return Err(ImportError::Geometry("Face material index is out of range"));
        }
        Ok(())
    }
}

impl Port {
    pub(crate) fn validate(&self) -> Result<()> {
        if ![&self.center, &self.outward, &self.up]
            .iter()
            .all(|p| finite_position(&p.components()))
            || !self.tunnel_depth.is_finite()
            || self.tunnel_depth <= 0.0
            || self.tunnel_depth > 1e12
            || self.aperture.len() < 3
            || self
                .aperture
                .iter()
                .any(|p| !finite_position(&p.components()))
        {
            return Err(ImportError::Geometry("Invalid finite station port data"));
        }
        let outward = self.outward.components();
        let up = self.up.components();
        let square = |v: [f64; 3]| v.iter().map(|x| x * x).sum::<f64>();
        let dot = outward.iter().zip(up).map(|(a, b)| a * b).sum::<f64>();
        if (square(outward) - 1.0).abs() > 1e-9
            || (square(up) - 1.0).abs() > 1e-9
            || dot.abs() > 1e-9
        {
            return Err(ImportError::Geometry(
                "Station port axes are not the selected orthonormal frame",
            ));
        }
        Ok(())
    }
}

pub(crate) fn display(mesh: &Design) -> Result<Vec<u8>> {
    mesh.validate()?;
    let mut bytes = Vec::new();
    for (face, &material) in mesh.faces.iter().zip(&mesh.face_materials) {
        let colour = mesh.materials[material as usize].colour;
        let linear = [24, 16, 8]
            .map(|shift| f32::from_bits(LINEAR_BITS[((colour >> shift) & 255) as usize]));
        for &index in face {
            for value in mesh.vertices[index as usize] {
                bytes.extend_from_slice(&(value as f32).to_le_bytes());
            }
            for value in linear {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
        }
    }
    Ok(bytes)
}

pub(crate) fn physical(mesh: &Design) -> Result<Vec<u8>> {
    mesh.validate()?;
    let count = |n: usize| {
        u32::try_from(n).map_err(|_| ImportError::Geometry("Physical mesh count exceeds LEu32"))
    };
    let mut bytes = b"MPHYS001".to_vec();
    bytes.extend_from_slice(&count(mesh.vertices.len())?.to_le_bytes());
    bytes.extend_from_slice(&count(mesh.faces.len())?.to_le_bytes());
    for position in &mesh.vertices {
        for value in position {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
    for face in &mesh.faces {
        for index in face {
            bytes.extend_from_slice(&index.to_le_bytes());
        }
    }
    Ok(bytes)
}

pub(crate) fn port(port: &Port) -> Result<Vec<u8>> {
    port.validate()?;
    let count = u32::try_from(port.aperture.len())
        .map_err(|_| ImportError::Geometry("Port count exceeds LEu32"))?;
    let mut bytes = b"MPORT001".to_vec();
    for point in [&port.center, &port.outward, &port.up] {
        for value in point.components() {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
    bytes.extend_from_slice(&port.tunnel_depth.to_le_bytes());
    bytes.extend_from_slice(&count.to_le_bytes());
    for point in &port.aperture {
        for value in point.components() {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
    Ok(bytes)
}
