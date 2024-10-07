use std::{env, fs};
use std::fs::File;
use std::io::Read;
use std::process::Command;
use ark_bn254::{Fq, Fq2, Fr, G1Projective, G2Projective};
use serde::{Deserialize, Serialize};
use anyhow::Result;
use ark_ff::PrimeField;
use ark_serialize::CanonicalSerialize;
use num_bigint::BigUint;
use num_traits::Num;
use hex;

fn to_aptos_move_bytes_expr<T: ark_serialize::CanonicalSerialize>(value: &T) -> String {
    let mut buf = vec![];
    value.serialize_compressed(&mut buf).unwrap();
    format!("x\"{}\"", hex::encode(buf))
}

fn to_aptos_move_bytes_arr_expr<T: ark_serialize::CanonicalSerialize>(values: &[T]) -> String {
    let mut items = vec![];
    for value in values {
        let bytes = to_aptos_move_bytes_expr(value);
        items.push(bytes);
    }
    let items_str = items.join(",");
    format!("vector[{}]", items_str)
}

type SnarkJsFrRepr = String;
fn try_as_fr(repr: &SnarkJsFqRepr) -> Result<Fr> {
    let bytes = BigUint::from_str_radix(repr.as_str(), 10)?.to_bytes_be();
    Ok(Fr::from_be_bytes_mod_order(&bytes))
}

type SnarkJsFqRepr = String;
fn try_as_fq(repr: &SnarkJsFqRepr) -> Result<Fq> {
    let bytes = BigUint::from_str_radix(repr.as_str(), 10)?.to_bytes_be();
    Ok(Fq::from_be_bytes_mod_order(&bytes))
}

type SnarkJsFq2Repr = [SnarkJsFqRepr; 2];
fn try_as_fq2(repr: &SnarkJsFq2Repr) -> Result<Fq2> {
    let x = try_as_fq(&repr[0])?;
    let y = try_as_fq(&repr[1])?;
    Ok(Fq2::new(x, y))
}

type SnarkJsG1Repr = [SnarkJsFqRepr; 3];
fn try_as_g1_proj(repr: &SnarkJsG1Repr) -> Result<G1Projective> {
    let a = try_as_fq(&repr[0])?;
    let b = try_as_fq(&repr[1])?;
    let c = try_as_fq(&repr[2])?;
    Ok(G1Projective::new(a, b, c))
}

type SnarkJsG2Repr = [SnarkJsFq2Repr; 3];
fn try_as_g2_proj(repr: &SnarkJsG2Repr) -> Result<G2Projective> {
    let a = try_as_fq2(&repr[0])?;
    let b = try_as_fq2(&repr[1])?;
    let c = try_as_fq2(&repr[2])?;
    Ok(G2Projective::new(a, b, c))
}

#[derive(Deserialize, Serialize)]
struct SnarkJsGroth16VerificationKey {
    vk_alpha_1: SnarkJsG1Repr,
    vk_beta_2: SnarkJsG2Repr,
    vk_gamma_2: SnarkJsG2Repr,
    vk_delta_2: SnarkJsG2Repr,
    #[serde(rename = "IC")]
    ic: Vec<SnarkJsG1Repr>,
}

#[derive(Deserialize, Serialize)]
struct SnarkJsGroth16Proof {
    pi_a: SnarkJsG1Repr,
    pi_b: SnarkJsG2Repr,
    pi_c: SnarkJsG1Repr,
}

type SnarkJsGroth16PublicInput = Vec<SnarkJsFrRepr>;

fn read_file_to_string(file_path: &str) -> Result<String> {
    let mut file = File::open(file_path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    Ok(content)
}

fn main() {
    let snarkjs_vk_path = std::env::var("IN_VK_PATH").unwrap();
    let snarkjs_public_input_path = std::env::var("IN_PUBLIC_INPUT_PATH").unwrap();
    let snarkjs_proof_path = std::env::var("IN_PROOF_PATH").unwrap();
    let out_dir = std::env::var("OUT_DIR").unwrap();

    let snarkjs_vk_str = read_file_to_string(&snarkjs_vk_path).unwrap();
    let snarkjs_vk: SnarkJsGroth16VerificationKey = serde_json::from_str(&snarkjs_vk_str).unwrap();

    let snarkjs_proof_str = read_file_to_string(&snarkjs_proof_path).unwrap();
    let snarkjs_proof: SnarkJsGroth16Proof = serde_json::from_str(&snarkjs_proof_str).unwrap();

    let snarkjs_public_input_str = read_file_to_string(&snarkjs_public_input_path).unwrap();
    let snarkjs_public_input: SnarkJsGroth16PublicInput = serde_json::from_str(&snarkjs_public_input_str).unwrap();
    let output = Command::new("bash")
        .envs(std::env::vars())
        .arg("-c")
        .arg(format!("rsync -a groth16_module_template/ {}/", out_dir))
        .output()
        .unwrap();
    assert!(output.status.success());

    let move_module_path = format!("{out_dir}/sources/groth16.move");
    let template = fs::read_to_string(&move_module_path).unwrap();
    let populated = template
        .replace("__VK_ALPHA_G1__", &to_aptos_move_bytes_expr(&try_as_g1_proj(&snarkjs_vk.vk_alpha_1).unwrap()))
        .replace("__VK_BETA_G2__", &to_aptos_move_bytes_expr(&try_as_g2_proj(&snarkjs_vk.vk_beta_2).unwrap()))
        .replace("__VK_GAMMA_G2__", &to_aptos_move_bytes_expr(&try_as_g2_proj(&snarkjs_vk.vk_gamma_2).unwrap()))
        .replace("__VK_DELTA_G2__", &to_aptos_move_bytes_expr(&try_as_g2_proj(&snarkjs_vk.vk_delta_2).unwrap()))
        .replace("__VK_GAMMA_ABC_G1__", &to_aptos_move_bytes_arr_expr(&snarkjs_vk.ic.iter().map(|x|try_as_g1_proj(x).unwrap()).collect::<Vec<_>>()))
        .replace("__VK_PUBLIC_INPUTS__", &to_aptos_move_bytes_arr_expr(&snarkjs_public_input.iter().map(|x|try_as_fr(x).unwrap()).collect::<Vec<_>>()))
        .replace("__PROOF_A__", &to_aptos_move_bytes_expr(&try_as_g1_proj(&snarkjs_proof.pi_a).unwrap()))
        .replace("__PROOF_B__", &to_aptos_move_bytes_expr(&try_as_g2_proj(&snarkjs_proof.pi_b).unwrap()))
        .replace("__PROOF_C__", &to_aptos_move_bytes_expr(&try_as_g1_proj(&snarkjs_proof.pi_c).unwrap()));

    fs::write(&move_module_path, populated).unwrap();
}
