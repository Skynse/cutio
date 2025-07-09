use std::error::Error;

use gst::prelude::*;
use gstreamer as gst;

/// Initializes GStreamer (safe to call multiple times).
fn ensure_gst_init() -> Result<(), Box<dyn Error>> {
    gst::init()?;
    Ok(())
}

/// Trims a video file using GStreamer.
///
/// # Arguments
/// * `input` - Path to the input video file.
/// * `output` - Path to the output trimmed video file.
/// * `start` - Start time in seconds.
/// * `end` - End time in seconds.
pub fn trim_video_gst(
    input: &str,
    output: &str,
    start: f64,
    end: f64,
) -> Result<(), Box<dyn Error>> {
    ensure_gst_init()?;

    // GStreamer pipeline for trimming video
    let pipeline_str = format!(
        "filesrc location=\"{}\" ! decodebin name=dec \
         dec. ! queue ! videoconvert ! x264enc ! mp4mux name=mux ! filesink location=\"{}\" \
         dec. ! queue ! audioconvert ! voaacenc ! mux.",
        input, output
    );
    let pipeline = gst::parse::launch(&pipeline_str)?;
    let pipeline = pipeline
        .downcast::<gst::Pipeline>()
        .expect("Expected a gst::Pipeline");

    // Set to PAUSED to preroll and allow seeking
    pipeline.set_state(gst::State::Paused)?;

    // Wait for preroll
    let bus = pipeline.bus().unwrap();
    loop {
        use gst::MessageView;
        match bus.timed_pop(gst::ClockTime::from_seconds(5)) {
            Some(msg) => match msg.view() {
                MessageView::AsyncDone(_) | MessageView::StateChanged(_) => break,
                MessageView::Error(err) => return Err(Box::new(err.error().clone())),
                _ => {}
            },
            None => break,
        }
    }

    // Seek to start and set stop at end
    let start_ns = (start * 1_000_000_000.0) as u64;
    let duration_ns = ((end - start) * 1_000_000_000.0) as u64;
    pipeline.seek(
        1.0,
        gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT,
        gst::SeekType::Set,
        gst::ClockTime::from_nseconds(start_ns),
        gst::SeekType::Set,
        gst::ClockTime::from_nseconds(start_ns + duration_ns),
    )?;

    // Set to Playing
    pipeline.set_state(gst::State::Playing)?;

    // Wait for EOS or Error
    for msg in bus.iter_timed(gst::ClockTime::NONE) {
        use gst::MessageView;
        match msg.view() {
            MessageView::Eos(..) => break,
            MessageView::Error(err) => return Err(Box::new(err.error().clone())),
            _ => (),
        }
    }

    pipeline.set_state(gst::State::Null)?;
    Ok(())
}

/// Concatenates multiple video files using GStreamer.
///
/// # Arguments
/// * `input_files` - Slice of paths to the video files to concatenate (in order).
/// * `output` - Path to the output concatenated video file.
pub fn concat_videos_gst(input_files: &[&str], output: &str) -> Result<(), Box<dyn Error>> {
    ensure_gst_init()?;

    let pipeline = gst::Pipeline::new();
    let concat = gst::ElementFactory::make("concat")
        .name("concat")
        .build()
        .expect("Failed to create concat");
    let videoconvert = gst::ElementFactory::make("videoconvert")
        .build()
        .expect("Failed to create videoconvert");
    let encoder = gst::ElementFactory::make("x264enc")
        .build()
        .expect("Failed to create x264enc");
    let muxer = gst::ElementFactory::make("mp4mux")
        .build()
        .expect("Failed to create mp4mux");
    let sink = gst::ElementFactory::make("filesink")
        .property("location", output)
        .build()
        .expect("Failed to create filesink");

    pipeline.add_many(&[&concat, &videoconvert, &encoder, &muxer, &sink])?;
    gst::Element::link_many(&[&concat, &videoconvert, &encoder, &muxer, &sink])?;

    for file in input_files {
        let src = gst::ElementFactory::make("filesrc")
            .property("location", file)
            .build()
            .expect("Failed to create filesrc");
        let decode = gst::ElementFactory::make("decodebin")
            .build()
            .expect("Failed to create decodebin");
        let queue = gst::ElementFactory::make("queue")
            .build()
            .expect("Failed to create queue");

        pipeline.add_many(&[&src, &decode, &queue])?;
        gst::Element::link_many(&[&src, &decode])?;

        let concat_clone = concat.clone();
        let queue_clone = queue.clone();
        decode.connect_pad_added(move |_dbin, src_pad| {
            let sink_pad = queue_clone.static_pad("sink").unwrap();
            if src_pad.link(&sink_pad).is_ok() {
                let _ = gst::Element::link_many(&[&queue_clone, &concat_clone]);
            }
        });
    }

    pipeline.set_state(gst::State::Playing)?;
    let bus = pipeline.bus().unwrap();

    for msg in bus.iter_timed(gst::ClockTime::NONE) {
        use gst::MessageView;
        match msg.view() {
            MessageView::Eos(..) => break,
            MessageView::Error(err) => return Err(Box::new(err.error().clone())),
            _ => (),
        }
    }

    pipeline.set_state(gst::State::Null)?;
    Ok(())
}

/// Trims an audio file using GStreamer.
///
/// # Arguments
/// * `input` - Path to the input audio file.
/// * `output` - Path to the output trimmed audio file.
/// * `start` - Start time in seconds.
/// * `end` - End time in seconds.
pub fn trim_audio_gst(
    input: &str,
    output: &str,
    start: f64,
    end: f64,
) -> Result<(), Box<dyn Error>> {
    ensure_gst_init()?;

    let pipeline_str = format!(
        "filesrc location=\"{}\" ! decodebin ! audioconvert ! voaacenc ! wavenc ! filesink location=\"{}\"",
        input, output
    );
    let pipeline = gst::parse::launch(&pipeline_str)?;
    let pipeline = pipeline
        .downcast::<gst::Pipeline>()
        .expect("Expected a gst::Pipeline");

    pipeline.set_state(gst::State::Paused)?;
    let bus = pipeline.bus().unwrap();
    loop {
        use gst::MessageView;
        match bus.timed_pop(gst::ClockTime::from_seconds(5)) {
            Some(msg) => match msg.view() {
                MessageView::AsyncDone(_) | MessageView::StateChanged(_) => break,
                MessageView::Error(err) => return Err(Box::new(err.error().clone())),
                _ => {}
            },
            None => break,
        }
    }

    let start_ns = (start * 1_000_000_000.0) as u64;
    let duration_ns = ((end - start) * 1_000_000_000.0) as u64;
    pipeline.seek(
        1.0,
        gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT,
        gst::SeekType::Set,
        gst::ClockTime::from_nseconds(start_ns),
        gst::SeekType::Set,
        gst::ClockTime::from_nseconds(start_ns + duration_ns),
    )?;

    pipeline.set_state(gst::State::Playing)?;

    for msg in bus.iter_timed(gst::ClockTime::NONE) {
        use gst::MessageView;
        match msg.view() {
            MessageView::Eos(..) => break,
            MessageView::Error(err) => return Err(Box::new(err.error().clone())),
            _ => (),
        }
    }

    pipeline.set_state(gst::State::Null)?;
    Ok(())
}

/// Mixes multiple audio files into one using GStreamer.
///
/// # Arguments
/// * `inputs` - Slice of paths to the audio files to mix.
/// * `output` - Path to the output mixed audio file.
pub fn mix_audio_gst(inputs: &[&str], output: &str) -> Result<(), Box<dyn Error>> {
    ensure_gst_init()?;

    let pipeline = gst::Pipeline::new();
    let mixer = gst::ElementFactory::make("audiomixer")
        .name("mixer")
        .build()
        .expect("Failed to create audiomixer");
    let audioconvert = gst::ElementFactory::make("audioconvert")
        .build()
        .expect("Failed to create audioconvert");
    let encoder = gst::ElementFactory::make("voaacenc")
        .build()
        .expect("Failed to create voaacenc");
    let wavenc = gst::ElementFactory::make("wavenc")
        .build()
        .expect("Failed to create wavenc");
    let sink = gst::ElementFactory::make("filesink")
        .property("location", output)
        .build()
        .expect("Failed to create filesink");

    pipeline.add_many(&[&mixer, &audioconvert, &encoder, &wavenc, &sink])?;
    gst::Element::link_many(&[&mixer, &audioconvert, &encoder, &wavenc, &sink])?;

    for input in inputs {
        let src = gst::ElementFactory::make("filesrc")
            .property("location", input)
            .build()
            .expect("Failed to create filesrc");
        let decode = gst::ElementFactory::make("decodebin")
            .build()
            .expect("Failed to create decodebin");
        let convert = gst::ElementFactory::make("audioconvert")
            .build()
            .expect("Failed to create audioconvert");
        let resample = gst::ElementFactory::make("audioresample")
            .build()
            .expect("Failed to create audioresample");
        let queue = gst::ElementFactory::make("queue")
            .build()
            .expect("Failed to create queue");

        pipeline.add_many(&[&src, &decode, &convert, &resample, &queue])?;
        gst::Element::link_many(&[&src, &decode])?;

        let mixer_clone = mixer.clone();
        let convert_clone = convert.clone();
        let resample_clone = resample.clone();
        let queue_clone = queue.clone();
        decode.connect_pad_added(move |_dbin, src_pad| {
            let sink_pad = convert_clone.static_pad("sink").unwrap();
            if src_pad.link(&sink_pad).is_ok() {
                let _ = gst::Element::link_many(&[
                    &convert_clone,
                    &resample_clone,
                    &queue_clone,
                    &mixer_clone,
                ]);
            }
        });
    }

    pipeline.set_state(gst::State::Playing)?;
    let bus = pipeline.bus().unwrap();

    for msg in bus.iter_timed(gst::ClockTime::NONE) {
        use gst::MessageView;
        match msg.view() {
            MessageView::Eos(..) => break,
            MessageView::Error(err) => return Err(Box::new(err.error().clone())),
            _ => (),
        }
    }

    pipeline.set_state(gst::State::Null)?;
    Ok(())
}

/// Muxes (combines) a video file and an audio file into a single output using GStreamer.
///
/// # Arguments
/// * `video` - Path to the video file.
/// * `audio` - Path to the audio file.
/// * `output` - Path to the output muxed file.
pub fn mux_audio_video_gst(video: &str, audio: &str, output: &str) -> Result<(), Box<dyn Error>> {
    ensure_gst_init()?;

    let pipeline_str = format!(
        "filesrc location=\"{}\" ! decodebin ! queue ! videoconvert ! x264enc ! mux. \
         filesrc location=\"{}\" ! decodebin ! queue ! audioconvert ! voaacenc ! mux. \
         mp4mux name=mux ! filesink location=\"{}\"",
        video, audio, output
    );
    let pipeline = gst::parse::launch(&pipeline_str)?;
    let pipeline = pipeline
        .downcast::<gst::Pipeline>()
        .expect("Expected a gst::Pipeline");

    pipeline.set_state(gst::State::Playing)?;
    let bus = pipeline.bus().unwrap();

    for msg in bus.iter_timed(gst::ClockTime::NONE) {
        use gst::MessageView;
        match msg.view() {
            MessageView::Eos(..) => break,
            MessageView::Error(err) => return Err(Box::new(err.error().clone())),
            _ => (),
        }
    }

    pipeline.set_state(gst::State::Null)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // These tests require GStreamer and valid test files.
    // Update the paths to valid files on your system to run.

    #[test]
    fn test_trim_video_gst() {
        let input = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/sample.mp4");
        let output =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/sample_trimmed.mp4");
        let input = input.to_str().unwrap();
        let output = output.to_str().unwrap();
        let start = 2.0;
        let end = 5.0;
        let result = trim_video_gst(input, output, start, end);
        assert!(result.is_ok());
        assert!(std::path::Path::new(output).exists());
        let _ = std::fs::remove_file(output);
    }

    #[test]
    fn test_concat_videos_gst() {
        let input1 = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/sample.mp4");
        let input2 = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/sample.mp4");
        let output =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/sample_concat.mp4");
        let input_files = vec![input1.to_str().unwrap(), input2.to_str().unwrap()];
        let output_str = output.to_str().unwrap();
        let result = concat_videos_gst(&input_files, output_str);
        assert!(result.is_ok());
        assert!(std::path::Path::new(output_str).exists());
        let _ = std::fs::remove_file(output_str);
    }

    #[test]
    fn test_trim_audio_gst() {
        let input = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/sample.wav");
        let output =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/sample_trimmed.wav");
        let input = input.to_str().unwrap();
        let output = output.to_str().unwrap();
        let start = 1.0;
        let end = 3.0;
        let result = trim_audio_gst(input, output, start, end);
        assert!(result.is_ok());
        assert!(std::path::Path::new(output).exists());
        let _ = std::fs::remove_file(output);
    }

    #[test]
    fn test_mix_audio_gst() {
        let input1 = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/sample.wav");
        let input2 = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/sample.wav");
        let output =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/sample_mixed.wav");
        let inputs = vec![input1.to_str().unwrap(), input2.to_str().unwrap()];
        let output_str = output.to_str().unwrap();
        let result = mix_audio_gst(&inputs, output_str);
        assert!(result.is_ok());
        assert!(std::path::Path::new(output_str).exists());
        let _ = std::fs::remove_file(output_str);
    }

    #[test]
    fn test_mux_audio_video_gst() {
        let video = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/sample.mp4");
        let audio = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/sample.wav");
        let output =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/sample_muxed.mp4");
        let video = video.to_str().unwrap();
        let audio = audio.to_str().unwrap();
        let output_str = output.to_str().unwrap();
        let result = mux_audio_video_gst(video, audio, output_str);
        assert!(result.is_ok());
        assert!(std::path::Path::new(output_str).exists());
        let _ = std::fs::remove_file(output_str);
    }
}
