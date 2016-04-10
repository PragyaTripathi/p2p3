extern crate gcc;

use std::process::Command;
use std::env;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::error::Error;
use std::path::PathBuf;
use std::path::Path;

fn make_temp_dir() -> PathBuf {
    let mut temp_dir_name = env::temp_dir();
    temp_dir_name.push("P2P3");
    fs::create_dir(temp_dir_name.as_path()).unwrap_or( () );
    //Return
    temp_dir_name
}

fn make_file(path: &Path, input: &str) -> Result<(), String>{
    let mut file = File::create(&path).unwrap_or_else(|e| panic!("Oh noooooo {}", e));
    match file.write_all(&input.as_bytes()){
        Err(why)=>{
            let display = path.display();
            return Err(format!("couldn't write to {}: {}", display,
                                               Error::description(&why)));
        },
        Ok(_)=>Ok( () )
    }
}

#[allow(dead_code)]
pub fn run_c(input: &str) -> Result<String, String> {
    let temp_dir_name = make_temp_dir();
    let c_file = temp_dir_name.join("temp_c_file.c");
    let exe = temp_dir_name.join("temp.exe");

    match make_file(&c_file, &input){
        Ok(_) =>{},
        Err(e) => return Err(e)
    };

    let output = match gcc::windows_registry::find("x86_64_msvc", "cl.exe"){
        Some(cc) => {
            let mut cc= cc;
            print!("{:?}", cc);
            cc.current_dir(temp_dir_name).arg(&c_file).arg(&format!("/link /OUT:{}", exe.to_str().unwrap())).output().unwrap() },
        None => Command::new("cc").current_dir(temp_dir_name).arg(&c_file).arg("-o").arg(&exe).output().unwrap()
    };

    if !output.status.success() {
        let a = output.status.code().unwrap();
        let b = String::from_utf8(output.stderr).unwrap();
        let c = String::from_utf8(output.stdout).unwrap();
        print!("Compile failed with code {}: {} {}", a, b, c);
        return Err(format!("Compile failed with code {}: {} {}", a, b, c));
    }

    let run_output = Command::new(exe).output().unwrap();

    if !run_output.status.success() {
        let a = run_output.status.code().unwrap();
        let b = String::from_utf8(run_output.stderr).unwrap();
        return Err(format!("Run failed with code {}: {}", a, b));
    }

    Ok(String::from_utf8(run_output.stdout).unwrap())
}

#[allow(dead_code)]
pub fn run_python(input: &str) -> Result<String, String> {
    let temp_dir = make_temp_dir();
    let py_file = temp_dir.join("temp_py.c");

    match make_file(&py_file, &input){
        Ok(_) =>{},
        Err(e) => return Err(e)
    };

    match Command::new("python").current_dir(temp_dir).arg(&py_file).output(){
        Ok(out) => {
            if out.status.success() {
                Ok(String::from_utf8(out.stdout).unwrap())
            }else{
                let a = out.status.code().unwrap();
                let b = String::from_utf8(out.stderr).unwrap();
                let c = String::from_utf8(out.stdout).unwrap();
                Err(format!("Python failed with code {}: {} {}", a, b, c))
            }
        },
        Err(err) => Err(format!("Failed to run python with error: {}",err))
    }
}

#[cfg(test)]
mod test{
    use super::*;

    static C_CODE: &'static str =
"#include <stdio.h>

void main(){
    printf(\"Hello World\");
    return 0;
}";

    static PY_CODE: &'static str =
"
print \"Hello World\",
";

    #[test]
    fn run_simple_c(){
        let out = run_c(C_CODE);

        print!("{:?}", out);
        match out{
            Err(_) => { assert!(false) },
            Ok(res) => {
                assert_eq!(res, "Hello World");
            }
        }
    }

    #[test]
    fn run_simple_py(){
        let out = run_python(PY_CODE);

        print!("{:?}", out);
        match out{
            Err(_) => { assert!(false) },
            Ok(res) => {
                assert_eq!(res.trim_right(), "Hello World");
            }
        }
    }
}
