use socketcan::{CanSocket, EmbeddedFrame, Frame, Socket};
use std::fs::File;
use std::io::BufReader;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct CanConfig {
    speed_index: usize,
    speed_gain: f32,
    brake_index: usize,
    shift_index: usize,
}

fn main() {
    println!("=== リアルタイムCAN常時監視システム（最上級） ===");

    // 1. JSON設定ファイルを読み込む（ブラックボックス）
    let config_file = File::open("config.json").expect("設定ファイルが開けませんでした");
    let config_reader = BufReader::new(config_file);
    let config: CanConfig = serde_json::from_reader(config_reader).expect("JSONのパースに失敗しました");

    // 2. 昨日開通させた仮想CANネットワーク「vcan0」に接続する
    println!("ネットワーク「vcan0」の監視を開始します...\n");
    let socket = CanSocket::open("vcan0").expect("vcan0 への接続に失敗しました");

    // 3. パケットが届くまで常時待機する無限ループ (C言語の while(1) )
    loop {
        // パケット（フレーム）を受信する
        let frame = socket.read_frame().expect("CANフレームの受信に失敗しました");

        // 届いたデータの生バイト配列を取得
        let packet = frame.data();
        if packet.len() < 3 { continue; }

        println!("--- [パケットリアルタイム受信] ---");

        // 4. 【車速のパース】
        let raw_speed = packet[config.speed_index] as f32;
        let speed_kmh = raw_speed * config.speed_gain;

        // 5. 【ブレーキのパース】
        let brake_bit = packet[config.brake_index];
        let is_brake_on = (brake_bit & 0x01) == 1;

        // 6. 【シフトの判定】
        let shift_bits = packet[config.shift_index];
        let shift_position = match shift_bits & 0x03 {
            0 => "P (パーキング)",
            1 => "R (リバース)",
            2 => "N (ニュートラル)",
            3 => "D (ドライブ)",
            _ => "不明",
        };

        // 7. 画面に出力
        println!("現在の車速: {} km/h", speed_kmh);
        println!("ブレーキ状態: {}", if is_brake_on { "ON" } else { "OFF" });
        println!("現在のシフト: {}", shift_position);

        // 8. 【安全診断ロジック】あなたの得意なC言語の領域です！
        if speed_kmh > 100.0 {
            println!("【⚠️警報】速度超過：100km/h を超えています！");
        }
        if (shift_bits & 0x03) == 2 && !is_brake_on {
            println!("【⚠️警告】Nレンジでブレーキが踏まれていません！");
        } else if (shift_bits & 0x03) == 2 && is_brake_on {
            println!("【ℹ️情報】Nレンジでブレーキ作動中。安全です。");
        }
        println!();
        // パケットを処理したあと、1秒（1000ミリ秒）待ってから次の受信に進む
        std::thread::sleep(std::time::Duration::from_millis(1000));
    }
}
