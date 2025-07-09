use std::fs::File;
use std::io::{self, Write};
use std::process::Command;

/// Trims a video file using ffmpeg CLI.
///
/// # Arguments
/// * `input` - Path to the input video file.
/// * `output` - Path to the output trimmed video file.
/// * `start` - Start time in seconds.
/// * `end` - End time in seconds.
///
/// # Panics
/// Panics if ffmpeg fails or returns a non-zero exit code.
pub fn trim_video_ffmpeg(input: &str, output: &str, start: f64, end: f64) -> io::Result<()> {
    let status = Command::new("ffmpeg")
        .args([
            "-y", // overwrite output
            "-i",
            input,
            "-ss",
            &format!("{}", start),
            "-to",
            &format!("{}", end),
            "-c",
            "copy",
            output,
        ])
        .status()?;
    if !status.success() {
        panic!("ffmpeg failed to trim video");
    }
    Ok(())
}

/// Concatenates multiple video files using ffmpeg CLI.
///
/// # Arguments
/// * `input_files` - Slice of paths to the video files to concatenate (in order).
/// * `output` - Path to the output concatenated video file.
///
/// # Panics
/// Panics if ffmpeg fails or returns a non-zero exit code.
pub fn concat_videos_ffmpeg(input_files: &[&str], output: &str) -> io::Result<()> {
    // Write the list of files to a temporary file in the format required by ffmpeg
    let mut list_file = tempfile::NamedTempFile::new()?;
    for file in input_files {
        writeln!(list_file, "file '{}'", file.replace("'", "'\\''"))?;
    }
    let list_path = list_file.path().to_str().unwrap();

    let status = Command::new("ffmpeg")
        .args([
            "-y", "-f", "concat", "-safe", "0", "-i", list_path, "-c", "copy", output,
        ])
        .status()?;
    if !status.success() {
        panic!("ffmpeg failed to concatenate videos");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // These tests are illustrative and will only pass if ffmpeg is installed and test files exist.
    // You may want to adapt them for your environment.

    #[test]
    fn test_trim_video_ffmpeg() {
        let input = "/home/neckles/Videos/car.mp4";
        let output = "/tmp/car_trimmed.mp4";
        let start = 2.0;
        let end = 5.0;
        let result = trim_video_ffmpeg(input, output, start, end);
        assert!(result.is_ok());
        assert!(std::path::Path::new(output).exists());
        // Optionally, clean up
        let _ = std::fs::remove_file(output);
    }

    #[test]
    fn test_concat_videos_ffmpeg() {
        let input_files = vec![
            "/home/neckles/Videos/car.mp4",
            "/home/neckles/Videos/car.mp4",
        ];
        let output = "/tmp/car_concat.mp4";
        let result = concat_videos_ffmpeg(&input_files, output);
        assert!(result.is_ok());
        assert!(std::path::Path::new(output).exists());
        let _ = std::fs::remove_file(output);
    }

    // To run this test, provide a valid WAV file path.
    // #[test]
    // fn test_trim_audio_ffmpeg() {
    //     let input = "/home/neckles/Videos/car.wav";
    //     let output = "/tmp/car_trimmed.wav";
    //     let start = 1.0;
    //     let end = 3.0;
    //     let result = trim_audio_ffmpeg(input, output, start, end);
    //     assert!(result.is_ok());
    //     assert!(std::path::Path::new(output).exists());
    //     let _ = std::fs::remove_file(output);
    // }

    // To run this test, provide valid WAV file paths.
    // #[test]
    // fn test_mix_audio_ffmpeg() {
    //     let inputs = vec!["/home/neckles/Videos/car.wav", "/home/neckles/Videos/car.wav"];
    //     let output = "/tmp/car_mixed.wav";
    //     let result = mix_audio_ffmpeg(&inputs, output);
    //     assert!(result.is_ok());
    //     assert!(std::path::Path::new(output).exists());
    //     let _ = std::fs::remove_file(output);
    // }

    // To run this test, provide valid MP4 and WAV file paths.
    // #[test]
    // fn test_mux_audio_video_ffmpeg() {
    //     let video = "/home/neckles/Videos/car.mp4";
    //     let audio = "/home/neckles/Videos/car.wav";
    //     let output = "/tmp/car_muxed.mp4";
    //     let result = mux_audio_video_ffmpeg(video, audio, output);
    //     assert!(result.is_ok());
    //     assert!(std::path::Path::new(output).exists());
    //     let _ = std::fs::remove_file(output);
    // }
}

/// Trims an audio file using ffmpeg CLI.
///
/// # Arguments
/// * `input` - Path to the input audio file.
/// * `output` - Path to the output trimmed audio file.
/// * `start` - Start time in seconds.
/// * `end` - End time in seconds.
///
/// # Panics
/// Panics if ffmpeg fails or returns a non-zero exit code.
pub fn trim_audio_ffmpeg(input: &str, output: &str, start: f64, end: f64) -> io::Result<()> {
    let status = Command::new("ffmpeg")
        .args([
            "-y",
            "-i",
            input,
            "-ss",
            &format!("{}", start),
            "-to",
            &format!("{}", end),
            "-c",
            "copy",
            output,
        ])
        .status()?;
    if !status.success() {
        panic!("ffmpeg failed to trim audio");
    }
    Ok(())
}

/// Mixes multiple audio files into one using ffmpeg CLI.
///
/// # Arguments
/// * `inputs` - Slice of paths to the audio files to mix.
/// * `output` - Path to the output mixed audio file.
///
/// # Panics
/// Panics if ffmpeg fails or returns a non-zero exit code.
pub fn mix_audio_ffmpeg(inputs: &[&str], output: &str) -> io::Result<()> {
    let mut args = vec!["-y"];
    for input in inputs {
        args.push("-i");
        args.push(input);
    }
    // Build amix filter for the number of inputs
    let filter = format!(
        "{}amix=inputs={}:duration=longest",
        (0..inputs.len())
            .map(|i| format!("[{}:a]", i))
            .collect::<Vec<_>>()
            .join(""),
        inputs.len()
    );
    args.extend_from_slice(&["-filter_complex", &filter, output]);
    let status = Command::new("ffmpeg").args(&args).status()?;
    if !status.success() {
        panic!("ffmpeg failed to mix audio");
    }
    Ok(())
}

/// Muxes (combines) a video file and an audio file into a single output using ffmpeg CLI.
///
/// # Arguments
/// * `video` - Path to the video file.
/// * `audio` - Path to the audio file.
/// * `output` - Path to the output muxed file.
///
/// # Panics
/// Panics if ffmpeg fails or returns a non-zero exit code.
pub fn mux_audio_video_ffmpeg(video: &str, audio: &str, output: &str) -> io::Result<()> {
    let status = Command::new("ffmpeg")
        .args([
            "-y",
            "-i",
            video,
            "-i",
            audio,
            "-c:v",
            "copy",
            "-c:a",
            "aac",
            "-shortest",
            output,
        ])
        .status()?;
    if !status.success() {
        panic!("ffmpeg failed to mux audio and video");
    }
    Ok(())
}
