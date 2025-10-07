use std::thread::JoinHandle;

use glam::Vec4;

pub struct ThreadPoolRenderer {
    worker_count: usize,
    workers: Vec<Worker>,
    frame_buffer: Vec<Vec4>,
}

pub struct Worker {
    join_handle: JoinHandle<()>,
}
