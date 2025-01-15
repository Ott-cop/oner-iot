use device::Device;
use esp_idf_svc::{eventloop::EspSystemEventLoop, hal::prelude::Peripherals, mqtt::client::{EspMqttClient, EspMqttConnection, EventPayload, MessageId, MqttClientConfiguration, QoS}, nvs::EspDefaultNvsPartition, sys::{ gpio_mode_t_GPIO_MODE_INPUT_OUTPUT, EspError}, wifi::{BlockingWifi, ClientConfiguration, EspWifi}};
use gpio::gpio::ControlGpio;
use log::info;
use serde::{Deserialize, Serialize};
use core::str;
use std::{sync::{Arc, Mutex}, thread, time::Duration};

pub mod gpio;
pub mod device;

const ENVCONFIGURATION: &str = include_str!("../env.json");

#[derive(Deserialize, Serialize)]
struct MsgReceived<'a> {
    id: MessageId,
    topic: Option<&'a str>,
    data: &'a [u8]
}

fn main() {
    let env_configuration = serde_json::from_str::<serde_json::Value>(ENVCONFIGURATION).unwrap();
    let wifi_ssid: &str = env_configuration["WIFI_SSID"].as_str().expect("Problem to convert WIFI_SSID to &str");
    let wifi_pass: &str = env_configuration["WIFI_PASS"].as_str().expect("Problem to convert WIFI_PASS to &str");
    
    let mqtt_url: &str = env_configuration["MQTT_URL"].as_str().expect("Problem to convert MQTT_URL to &str");
    let mqtt_client_id: &str = env_configuration["MQTT_CLIENT_ID"].as_str().unwrap();
    let mqtt_client_pass: &str = env_configuration["MQTT_CLIENT_PASS"].as_str().unwrap();
    
    esp_idf_svc::sys::link_patches();

    esp_idf_svc::log::EspLogger::initialize_default();
    
    unsafe {
        esp_idf_svc::sys::esp_task_wdt_deinit();
        esp_idf_svc::sys::esp_wifi_set_ps(esp_idf_svc::sys::wifi_ps_type_t_WIFI_PS_NONE);
    }

    let sysloop = EspSystemEventLoop::take().unwrap();
    let nvs = EspDefaultNvsPartition::take().unwrap();

    let _wifi_create = wifi_create(&sysloop, &nvs, wifi_ssid, wifi_pass).expect("Unable to configure WIFI");
    info!("Wifi Created!");

    let (client, conn) = mqtt_create(mqtt_url, mqtt_client_id, mqtt_client_pass).unwrap();
    info!("MQTT Client Created!");

    let client = Arc::new(Mutex::new(client));
    let conn = Arc::new(Mutex::new(conn));

    let cli1 = Arc::clone(&client);
    let conn1 = Arc::clone(&conn);
    
    exec(cli1, conn1, Arc::new(String::from("pin_2")), ControlGpio::new(2, gpio_mode_t_GPIO_MODE_INPUT_OUTPUT));
    
    loop {}
}

fn exec(
    client: Arc<Mutex<EspMqttClient<'static>>>,
    conn: Arc<Mutex<EspMqttConnection>>,
    subscribe_topic: Arc<String>,
    gpio: ControlGpio
) {
    let st = Arc::clone(&subscribe_topic);
    let st2 = Arc::clone(&subscribe_topic);
    thread::spawn(move || sub_publish(client, st));
    thread::spawn(move || sub_receive(conn, st2, gpio));
}


fn sub_receive(
    conn: Arc<Mutex<EspMqttConnection>>,
    _topic: Arc<String>,
    gpio: ControlGpio
) -> () {
    let mut conn = conn.lock().unwrap();
    info!("MQTT Listening for messages");

    while let Ok(event) = conn.next() {
        if let EventPayload::Received{ id, topic, data, details } = event.payload() {
            let raw_json = str::from_utf8(data).unwrap();
            info!("ID > {}", id);
            info!("Topic > {:#?}", topic);
            info!("Data > {}", raw_json);
            info!("Details > {:#?}", details);

            if let Ok(device_recv) = serde_json::from_str::<Device>(raw_json) {
                println!("{}", device_recv.state);
                let mut device = Device {
                    id: device_recv.id,
                    name: device_recv.name,
                    state: device_recv.state
                };

                if topic == Some(_topic.as_str()) {
                
                    if device_recv.state == true {
                        if let Err(err) = gpio.set_value(1) {
                            info!("{err}");
                            continue;
                        } 
                        device.state = true;
                    } else {
                        if let Err(err) = gpio.set_value(0) {
                            info!("{err}");
                            continue;
                        } 
                        device.state = false;
                    }

                    info!("Topic setted: {} -> {}", device.id, device.state);   
                }
            }
        } else if let EventPayload::Error(err) = event.payload() {
            info!("Problem to connect: {err}");
        }
    }
}

fn sub_publish(
    client: Arc<Mutex<EspMqttClient<'_>>>,
    subscribe_topic: Arc<String>
) {
    let mut client = client.lock().unwrap();
    loop {
        thread::sleep(Duration::from_millis(100));
        
        if let Ok(_) = client.subscribe(subscribe_topic.as_str(), QoS::AtMostOnce) {
            break;
        } else {
            info!("Failed to subscribe to topic {}", subscribe_topic);
            thread::sleep(Duration::from_millis(5000));
            continue;
        }
          
    }
    info!("Subscribed to topic: {}", subscribe_topic);  
    
}

fn mqtt_create(
    url: &str,
    client_id: &str,
    client_pass: &str
) -> Result<(EspMqttClient<'static>, EspMqttConnection), EspError> {
    
    let (mqtt_client, mqtt_conn) = EspMqttClient::new(url, &MqttClientConfiguration {
        client_id: Some(client_id),
        username: Some(client_id),
        password: Some(client_pass),
        reconnect_timeout: Some(Duration::from_secs(5)),
        client_certificate: None,
        server_certificate: None,
        ..Default::default()
    }).unwrap();

    println!("{} {}", client_id, client_pass);

    Ok((mqtt_client, mqtt_conn))
}


fn wifi_create(
    sysloop: &EspSystemEventLoop,
    nvs: &EspDefaultNvsPartition,
    wifi_ssid: &str,
    wifi_pass: &str

) -> Result<EspWifi<'static>, EspError> {
    let peripherals = Peripherals::take().unwrap();

    let mut esp_wifi = EspWifi::new(peripherals.modem, sysloop.clone(), Some(nvs.clone())).unwrap();

    let mut wifi = BlockingWifi::wrap(&mut esp_wifi, sysloop.clone()).unwrap();

    wifi.set_configuration(&esp_idf_svc::wifi::Configuration::Client(ClientConfiguration {
        ssid: wifi_ssid.try_into().unwrap(),
        password: wifi_pass.try_into().unwrap(),
        auth_method: esp_idf_svc::wifi::AuthMethod::WPA2Personal,
        ..Default::default()
    })).unwrap();

    wifi.start().expect("Unable to start wifi!");
    info!("Starting wifi...");

    wifi.connect().expect("Unable to connect to wifi");
    info!("Connecting to {}.", wifi_ssid);

    wifi.wait_netif_up().expect("Timeout connection!");
    info!("Wifi Connected!");

    Ok(esp_wifi)
}

