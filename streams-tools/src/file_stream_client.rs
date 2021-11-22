use iota_streams::{
    app::transport::{
        Transport,
        TransportDetails,
        TransportOptions,
        tangle::{
            TangleAddress,
            TangleMessage,
            client::{
                Client,
                Details,
                SendOptions,
            }
        },
    },
    core::{
        async_trait,
        Result,
    },
};

use std::{
    fs::{
        File,
        read_dir,
    },
    path::Path,
    marker::PhantomData,
    clone::Clone,
    io::{
        Read,
        Write,
        BufReader,
        BufWriter,
    },
};
use anyhow::Context;

use iota_streams::app::message::{BinaryMessage, BinaryBody};

pub trait WrappedClient {
    fn new_from_url(url: &str) -> Self;
}

#[derive(Clone)]
pub struct FileStreamClient<F> {
    phantom: PhantomData<F>,
    client: Client,
    number_of_written_files: u32,
    pub input_folder_path: String,
    pub output_folder_path: String,
}

impl<F> WrappedClient for FileStreamClient<F>
    where
        F: 'static + core::marker::Send + core::marker::Sync,
{
    fn new_from_url(url: &str) -> Self {
        Self {
            phantom: PhantomData,
            client: Client::new_from_url(url),
            number_of_written_files: 0,
            input_folder_path: String::from(""),
            output_folder_path: String::from(""),
        }
    }
}

impl<F> FileStreamClient<F>
    where
        F: 'static + core::marker::Send + core::marker::Sync,
{
    fn create_output_file(&mut self, link: &TangleAddress) -> Result<File> {
        let file_path_and_name = String::from(format!("{}/msg_{:04}-{}",
                                                      self.input_folder_path,
                                                      self.number_of_written_files,
                                                      link.to_string()
        ));
        let file = File::create(file_path_and_name.as_str()).context(format!("Create output file '{}'", file_path_and_name))?;
        Ok(file)
    }

    fn write_message_to_file(&mut self, msg: &TangleMessage<F>) -> Result<()> {
        let link = msg.binary.link;
        let out_file = self.create_output_file(&link)?;
        let mut writer = BufWriter::new(out_file);
        writer.write_all(msg.binary.body.bytes.as_slice())?;
        self.number_of_written_files += 1;
        Ok(())
    }

    fn search_file_in_input_folder(&mut self, link: &TangleAddress) -> Result<File> {
        let path_obj = Path::new(self.input_folder_path.as_str());
        if !path_obj.is_dir() {
            panic!("Path '{}' does not exist or is not a directory.", self.input_folder_path);
        }
        let input_dir = read_dir(path_obj)?;
        for entry in input_dir {
            let entry = entry?;
            if let Some(file_name) = entry.file_name().to_str() {
                if file_name.contains(&link.to_string()) {
                    let file_result = File::open(file_name)?;
                    return Ok(file_result);
                }
            } else {
                println!("[CaptureClient.send_message] Could not find file name");
            }
        }
        panic!("Could not find message file for address {}", link.to_string());
    }

    fn read_message_from_file(&mut self, link: &TangleAddress) -> Result<TangleMessage<F>> {
        let f = self.search_file_in_input_folder(link)?;
        let mut reader = BufReader::new(f);
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer)?;

        let empty_address = TangleAddress::default();
        let binary_msg = BinaryMessage::new(empty_address, *link, BinaryBody::from(buffer));
        Ok(TangleMessage::new(binary_msg))
    }
}

#[async_trait(?Send)]
impl<F> Transport<TangleAddress, TangleMessage<F>> for FileStreamClient<F>
    where
        F: 'static + core::marker::Send + core::marker::Sync,
{
    async fn send_message(&mut self, msg: &TangleMessage<F>) -> Result<()> {
        println!("[CaptureClient.send_message] Sending message with {} bytes payload:\n{}\n", msg.binary.body.bytes.len(), msg.binary.to_string());
        self.write_message_to_file(msg)
    }

    async fn recv_messages(&mut self, link: &TangleAddress) -> Result<Vec<TangleMessage<F>>> {
        let ret_val = self.client.recv_messages(link).await;
        match ret_val.as_ref() {
            Ok(msg_vec) => {
                for (idx, msg) in msg_vec.iter().enumerate() {
                    println!("[CaptureClient.recv_messages] - idx {}: Receiving message with {} bytes payload:\n{}\n", idx, msg.binary.body.bytes.len(), msg.binary.to_string())
                }
            },
            _ => ()
        }
        ret_val
    }

    async fn recv_message(&mut self, link: &TangleAddress) -> Result<TangleMessage<F>> {
        let ret_val = self.read_message_from_file(link);
        match ret_val.as_ref() {
            Ok(msg) => println!("[CaptureClient.recv_message] Receiving message with {} bytes payload:\n{}\n", msg.binary.body.bytes.len(), msg.binary.to_string()),
            _ => ()
        }
        ret_val
    }
}

#[async_trait(?Send)]
impl<F> TransportDetails<TangleAddress> for FileStreamClient<F> {
    type Details = Details;
    async fn get_link_details(&mut self, link: &TangleAddress) -> Result<Self::Details> {
        self.client.get_link_details(link).await
    }
}

impl<F> TransportOptions for FileStreamClient<F> {
    type SendOptions = SendOptions;
    fn get_send_options(&self) -> SendOptions {
        self.client.get_send_options()
    }
    fn set_send_options(&mut self, opt: SendOptions) {
        self.client.set_send_options(opt)
    }

    type RecvOptions = ();
    fn get_recv_options(&self) {}
    fn set_recv_options(&mut self, _opt: ()) {}
}
