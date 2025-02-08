use std::sync::{Arc, Mutex};

pub enum Verdict {
    Drop,
    Accept,
}

pub struct NfqPacket {
    pub id: u32,
    pub payload: Vec<u8>,
    qqh: Arc<Mutex<NfqueueQueueHandle>>,
}

unsafe impl Send for NfqPacket {}

impl NfqPacket {
    pub fn get_payload(&self) -> &[u8] {
        self.payload.as_slice()
    }

    pub fn set_verdict(&self, verdict: Verdict) {
        let c_verdict: u32 = match verdict {
            Verdict::Accept => 1,
            Verdict::Drop => 0,
        };
        log::debug!("set verdict {}, {}", self.id, c_verdict);
        let qqh = self.qqh.lock().unwrap();
        unsafe { nfq_set_verdict2(*qqh, self.id, c_verdict, 0, 0, std::ptr::null_mut()) }
    }
}

pub type Callback<T> = fn(NfqPacket, &mut T) -> ();

pub struct NfQueueWrapper<T> {
    qh: NfqueueHandle,
    cb: Callback<T>,
    data: T,
    pub qqh: Arc<Mutex<NfqueueQueueHandle>>,
}

impl<T: Send> NfQueueWrapper<T> {
    #[allow(clippy::mutex_atomic)]
    pub fn new(data: T, cb: Callback<T>) -> Self {
        let qh = unsafe { nfq_open() };

        if qh.is_null() {
            panic!("Error in nfq_open");
        }

        NfQueueWrapper {
            qh,
            cb,
            data,
            qqh: Arc::new(Mutex::new(std::ptr::null_mut())),
        }
    }

    #[allow(clippy::mutex_atomic)]
    pub fn open(&mut self, queue_num: u16) {
        log::info!("open nfqueue wrapper, queue number: {}", queue_num);

        unsafe { nfq_unbind_pf(self.qh, libc::AF_INET) };
        unsafe { nfq_bind_pf(self.qh, libc::AF_INET) };

        let self_ptr = unsafe { std::mem::transmute(&*self) };
        let qqh = unsafe { nfq_create_queue(self.qh, queue_num, nfq_callback::<T>, self_ptr) };

        if qqh.is_null() {
            panic!("Error in nfq_create_queue for queue {}, wrong queue number or insufficient privileges", queue_num);
        }

        self.qqh = Arc::new(Mutex::new(qqh));
        unsafe { nfq_set_mode(qqh, NFQNL_COPY_PACKET, 0xfffff) };
        unsafe { nfq_set_queue_maxlen(qqh, 1024 * 1024 * 1024) };
    }

    pub fn run_loop(&mut self) {
        let fd = unsafe { nfq_fd(self.qh) };
        let mut buf: [u8; 1024 * 1024] = [0; 1024 * 1024];
        let buf_ptr = buf.as_mut_ptr() as *mut libc::c_void;
        let buf_len = buf.len() as libc::size_t;

        loop {
            let rc = unsafe { libc::recv(fd, buf_ptr, buf_len, 0) };
            if rc < 0 {
                log::error!("received error code {}", rc);
            }

            unsafe { nfq_handle_packet(self.qh, buf_ptr, rc as libc::c_int) };
        }
    }
}

// C stuff
type NfqueueHandle = *const libc::c_void;
type NfqueueQueueHandle = *const libc::c_void;
type NfqueueCCallback = extern "C" fn(
    *const libc::c_void,
    *const libc::c_void,
    *const libc::c_void,
    *const libc::c_void,
);
type NfqueueData = *const libc::c_void;

/// Metaheader wrapping a packet
#[repr(C)]
pub struct NfMsgPacketHdr {
    /// unique ID of the packet
    pub packet_id: u32,
    /// hw protocol (network order)
    pub hw_protocol: u16,
    /// Netfilter hook
    pub hook: u8,
}

#[link(name = "netfilter_queue")]
extern "C" {
    // library setup
    fn nfq_open() -> NfqueueHandle;
    //fn nfq_close(qh: NfqueueHandle);
    fn nfq_bind_pf(qh: NfqueueHandle, pf: libc::c_int) -> libc::c_int;
    fn nfq_unbind_pf(qh: NfqueueHandle, pf: libc::c_int) -> libc::c_int;

    // queue handling
    fn nfq_fd(h: NfqueueHandle) -> libc::c_int;
    fn nfq_create_queue(
        qh: NfqueueHandle,
        num: u16,
        cb: NfqueueCCallback,
        data: *mut libc::c_void,
    ) -> NfqueueQueueHandle;
    //fn nfq_destroy_queue(qh: NfqueueHandle) -> libc::c_int;
    fn nfq_handle_packet(qh: NfqueueHandle, buf: *mut libc::c_void, rc: libc::c_int)
        -> libc::c_int;
    fn nfq_set_mode(gh: NfqueueQueueHandle, mode: u8, range: u32) -> libc::c_int;
    fn nfq_set_queue_maxlen(gh: NfqueueQueueHandle, queuelen: u32) -> libc::c_int;

    fn nfq_set_verdict2(
        qqh: *const libc::c_void,
        id: u32,
        verdict: u32,
        mark: u32,
        data_len: u32,
        data: *const libc::c_uchar,
    );

    // message parsing functions
    fn nfq_get_msg_packet_hdr(nfad: NfqueueData) -> *const libc::c_void;
    fn nfq_get_payload(nfad: NfqueueData, data: &*mut libc::c_void) -> libc::c_int;
}

#[doc(hidden)]
extern "C" fn nfq_callback<T>(
    _qqh: *const libc::c_void,
    _nfmsg: *const libc::c_void,
    nfad: *const libc::c_void,
    data: *const libc::c_void,
) {
    let raw: *mut NfQueueWrapper<T> = data as *mut NfQueueWrapper<T>;
    let q = &mut unsafe { &mut *raw };

    let msg_hdr = unsafe { nfq_get_msg_packet_hdr(nfad) as *const NfMsgPacketHdr };

    let c_ptr = std::ptr::null_mut();
    let payload_len = unsafe { nfq_get_payload(nfad, &c_ptr) };
    let payload: &[u8] =
        unsafe { std::slice::from_raw_parts(c_ptr as *mut u8, payload_len as usize) };

    let msg = NfqPacket {
        id: u32::from_be(unsafe { (*msg_hdr).packet_id }),
        qqh: Arc::clone(&q.qqh),
        payload: payload.to_vec(),
    };

    let callback = q.cb;
    callback(msg, &mut q.data);
}

const NFQNL_COPY_PACKET: u8 = 0x02;
