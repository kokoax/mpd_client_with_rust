extern crate regex;

use std;
use std::io::{Read, Write};
use std::vec::Vec;
use std::string::String;
use std::collections::HashMap;
use std::net::{TcpStream, Ipv4Addr,SocketAddrV4};
use std::sync::Mutex;

pub struct MPDQuery {
    mpd:  Mutex<TcpStream>,
    addr: Ipv4Addr,
    port: u16
}

impl Clone for MPDQuery {
    fn clone(&self) -> MPDQuery {
        MPDQuery::new(self.addr, self.port)
    }
}

impl MPDQuery {
    pub fn new(addr: Ipv4Addr, port: u16) -> MPDQuery {
        let mut mpd: TcpStream = TcpStream::connect(SocketAddrV4::new(addr, port))
            .expect("Failed get TCP socket(MPDQuery::get_mpd_socket)");

        // TODO without timeout
        // So, specify end keyword such.
        let _ = mpd.set_read_timeout(Some(std::time::Duration::new(0,1)));

        // Receive message("OK MPD $mpd_version") from mpd when after connect mpd, that throw to dustbox.
        let mut buf: String = String::new();
        let _ = mpd.read_to_string(&mut buf);

        let mutex_mpd = Mutex::<TcpStream>::new(mpd);
        MPDQuery{mpd: mutex_mpd, addr: addr, port: port}
    }

    // MPD receive data(String) to vector<hashmap>
    fn mpdbuf_to_vec(&self, buf: String) -> Vec<HashMap<String, String>> {
        let mut ret: Vec<HashMap<String, String>> = Vec::<HashMap<String, String>>::new();
        let mut ls: Vec<&str> = buf.split("\n").collect();
        ls.pop();  // "OK\n".split("\n") -> ["OK", ""].pop()

        let mut ls_data: HashMap<String, String> = HashMap::<String, String>::new();

        // The buffer's last line is "OK"
        let is_last = regex::Regex::new(r"OK").unwrap();
        // The buffer's top line is "file" or "directory" or "playlist"
        let is_top_attr = regex::Regex::new(r"file|directory|playlist").unwrap();
        for line in ls {
            if !is_last.is_match(line) {
                // ex:item. "file: ~/Music/Sample.mp3".splite(": ") -> ["file", "~/Music/Sample.mp3"]
                let splited: Vec<&str> = line.split(": ").collect();
                if is_top_attr.is_match(splited[0]) {
                    ret.push(ls_data.clone());
                    ls_data.clear();
                }
                ls_data.insert(splited[0].to_string(), splited[1].to_string());
            }else{
                ret.push(ls_data.clone());
            }
        }
        ret.remove(0);
        return ret;
    }

    pub fn status(&self) -> HashMap<String, String> {
        let buf = self.do_cmd(format!("status\n"));
        return self.mpdbuf_to_vec(buf).pop().unwrap();
    }

    pub fn delete(&self, songpos: &str){
        // TODO: catch Exeption and return error or expect
        let _ = self.do_cmd(format!("delete \"{}\"\n", songpos));
    }

    pub fn add(&self, uri: &str){
        let _ = self.do_cmd(format!("add \"{}\"\n", uri));
    }

    // get currentsong infomation
    pub fn currentsong(&self) -> HashMap<String, String> {
        let  buf: String = self.do_cmd(format!("currentsong\n"));

        return self.mpdbuf_to_vec(buf).pop().unwrap();
    }

    // get list any types(song, album, artist, etc...)
    pub fn list(&self, filter: &str) -> Vec<String> {
        let  buf: String = self.do_cmd(format!("list {}\n", filter));

        let splited: Vec<&str> = buf.split("\n").collect();

        let mut ret: Vec<String> = splited.into_iter().map(|x| {
            match x {
                "OK" => "OK".to_string(),
                "" => "NL".to_string(),
                _      => {
                    let splited: Vec<&str> = x.split(": ").collect();
                    splited[1].to_string()
                },
            }
        }).collect();
        ret.remove(0);
        ret.pop();
        ret.pop();
        return ret;
    }

    pub fn listall(&self) -> Vec<HashMap<String, String>> {
        let  buf: String = self.do_cmd(format!("listall\n"));

        return self.mpdbuf_to_vec(buf);
    }

    pub fn playlistinfo(&self, songpos: &str) -> Vec<HashMap<String, String>> {
        let  buf: String = self.do_cmd(format!("playlistinfo {}\n", songpos));

        return self.mpdbuf_to_vec(buf);
    }

    pub fn playlist(&self) -> Vec<HashMap<String, String>> {
        let  buf: String = self.do_cmd(format!("playlist\n"));

        return self.mpdbuf_to_vec(buf);
    }

    pub fn find(&self, filter: &str, uri: &str) -> Vec<HashMap<String, String>> {
        let  buf: String = self.do_cmd(format!("find {} \"{}\"\n", filter, uri));

        return self.mpdbuf_to_vec(buf);
    }

    // get only directory from ls
    pub fn ls_dir(&self, path: &str) -> Vec<HashMap<String, String>> {
        let mut ls_dir: Vec<HashMap<String, String>> = self.ls(path);
        ls_dir.retain(|item| item.contains_key("directory"));
        return ls_dir;
    }
    // get only directory from ls
    pub fn ls_song(&self, path: &str) -> Vec<HashMap<String, String>> {
        let mut ls_dir_and_song: Vec<HashMap<String, String>> = self.ls(path);
        ls_dir_and_song.retain(|item| item.contains_key("file"));
        return ls_dir_and_song;
    }
    // get only playlist from ls
    pub fn ls_playlist(&self, path: &str) -> Vec<HashMap<String, String>> {
        let mut ls_playlist: Vec<HashMap<String, String>> = self.ls(path);
        ls_playlist.retain(|item| item.contains_key("playlist"));
        return ls_playlist;
    }
    // get directory and song from ls
    pub fn ls_dir_and_song(&self, path: &str) -> Vec<HashMap<String, String>> {
        let mut ls_dir_and_song: Vec<HashMap<String, String>> = self.ls(path);
        ls_dir_and_song.retain(|item| item.contains_key("file") || item.contains_key("directory"));
        return ls_dir_and_song;
    }
    // get mpd' ls command result
    pub fn ls(&self, path: &str) -> Vec<HashMap<String, String>> {
        let  buf: String = self.do_cmd(format!("lsinfo \"{}\"\n", path));

        return self.mpdbuf_to_vec(buf);
    }

    fn do_cmd(&self, cmd: String) -> String {
        let mut mpd = self.mpd.lock().unwrap();
        let mut buf: String = String::new();
        let is_ending = regex::Regex::new(r"OK").unwrap();

        let _ = mpd.write(cmd.as_bytes());
        let _ = mpd.read_to_string(&mut buf);

        while !is_ending.is_match(&buf) {
            std::thread::sleep(std::time::Duration::from_millis(1));
            let mut tmp: String = String::new();
            let _ = mpd.read_to_string(&mut tmp);
            buf = format!("{}{}", buf, tmp);
        }

        return buf;
    }
}

