use apeireth_voice::{LipSyncCalculator, TtsClient, TtsConfig, VadDetector, VadConfig, VadState, AudioChunkStreamer};

#[test]
fn test_lipsync_silence_and_audio() {
    let calc = LipSyncCalculator::new(24000);
    let silent_samples = vec![0i16; 480];
    let frame = calc.process_chunk(0, &silent_samples);
    assert_eq!(frame.mouth_open_y, 0.0);

    let loud_samples: Vec<i16> = (0..480).map(|i| ((i as f32 * 0.1).sin() * 20000.0) as i16).collect();
    let loud_frame = calc.process_chunk(20, &loud_samples);
    assert!(loud_frame.mouth_open_y > 0.3);
}

#[tokio::test]
async fn test_tts_synthesis() {
    let client = TtsClient::new(TtsConfig::default());
    let ssml = client.build_ssml("你好，我是 Apeireth！");
    assert!(ssml.contains("Apeireth"));

    let audio = client.synthesize("测试语音").await.unwrap();
    assert!(!audio.is_empty());
}

#[test]
fn test_vad_interruption() {
    let mut vad = VadDetector::new(VadConfig::default());
    let (s1, int1) = vad.process_energy(0.01);
    assert_eq!(s1, VadState::Silence);
    assert!(!int1);

    vad.process_energy(0.1);
    vad.process_energy(0.1);
    let (s3, int3) = vad.process_energy(0.1);
    assert_eq!(s3, VadState::Speaking);
    assert!(int3); // Triggers interruption
}

#[test]
fn test_audio_chunk_streamer() {
    let mut streamer = AudioChunkStreamer::new(24000);
    let pcm = vec![0u8; 960];
    let frame = streamer.process_raw_audio(100, &pcm, true);
    assert_eq!(frame.sequence_id, 1);
    assert!(frame.is_final);
}
