use core::arch::naked_asm;
const STACK_SIZE: usize = 1024 * 8;
const MAX_THREADS: usize = 16;
static mut RUNTIME_ADDR: usize = 0;

#[derive(Debug, Default)]
#[repr(C)]
struct ThreadContext {
    rsp: u64,
    r15: u64,
    r14: u64,
    r13: u64,
    r12: u64,
    rbx: u64,
    rbp: u64,
}

#[unsafe(naked)]
#[unsafe(no_mangle)]
unsafe extern "C" fn switch(old: *mut ThreadContext, new: *const ThreadContext) {
    naked_asm!(
        "mov [rdi + 0x00], rsp",
        "mov [rdi + 0x08], r15",
        "mov [rdi + 0x10], r14",
        "mov [rdi + 0x18], r13",
        "mov [rdi + 0x20], r12",
        "mov [rdi + 0x28], rbx",
        "mov [rdi + 0x30], rbp",
        "mov rsp, [rsi + 0x00]",
        "mov r15, [rsi + 0x08]",
        "mov r14, [rsi + 0x10]",
        "mov r13, [rsi + 0x18]",
        "mov r12, [rsi + 0x20]",
        "mov rbx, [rsi + 0x28]",
        "mov rbp, [rsi + 0x30]",
        "ret"
    );
}

#[derive(Debug, PartialEq, Eq)]
enum State {
    Available,
    Running,
    Ready,
}
#[derive(Debug)]
struct Thread {
    id: usize,
    stack: Vec<usize>,
    ctx: ThreadContext,
    state: State,
}
impl Thread {
    pub fn new(id: usize) -> Self {
        Self {
            id,
            stack: vec![0; STACK_SIZE],
            ctx: ThreadContext::default(),
            state: State::Available,
        }
    }
}

#[derive(Debug)]
struct Runtime {
    threads: Vec<Thread>,
    current: usize,
}

fn guard() {
    let rt = Runtime::from_addr(unsafe { RUNTIME_ADDR });
    if rt.current != 0 {
        rt.threads[rt.current].state = State::Available;
    }
    rt.yield_thread();
}

#[unsafe(naked)]
unsafe extern "C" fn skip() {
    naked_asm!("ret");
}

impl Runtime {
    pub fn spawn(&mut self, f: fn()) {
        let available = self
            .threads
            .iter_mut()
            .find(|t| t.state == State::Available)
            .expect("no available");
        let size = available.stack.len();

        unsafe {
            let s_ptr = available.stack.as_mut_ptr().add(size);
            let s_ptr = (s_ptr as usize & !0xf) as *mut u8; // align
            core::ptr::write(s_ptr.offset(-0x10) as *mut u64, guard as u64);
            core::ptr::write(s_ptr.offset(-0x18) as *mut u64, skip as u64);
            core::ptr::write(s_ptr.offset(-0x20) as *mut u64, f as u64);
            available.ctx.rsp = s_ptr.offset(-0x20) as u64;
        }
        available.state = State::Ready;
    }
    #[inline(never)]
    pub fn yield_thread(&mut self) -> bool {
        let mut next = self.current;
        let current = self.current;
        while self.threads[next].state != State::Ready {
            next += 1;
            if next == self.threads.len() {
                next = 0;
            }
            if next == current {
                return false;
            }
        }

        if self.threads[current].state != State::Available {
            // println!("Ready {}", current);
            self.threads[current].state = State::Ready;
        }

        // println!("Running {}", next);
        self.threads[next].state = State::Running;
        self.current = next;

        unsafe {
            let old: *mut ThreadContext = &mut self.threads[current].ctx;
            let new: *const ThreadContext = &self.threads[next].ctx;
            switch(old, new);
        }

        !self.threads.is_empty()
    }
    pub fn run(&mut self) {
        println!("start");
        while self.yield_thread() {}
    }
    pub fn get_addr(&self) -> usize {
        let ptr: *const Self = self;
        ptr as usize
    }
    pub fn new() -> Self {
        let base_thread = Thread {
            id: 0,
            stack: vec![0; STACK_SIZE],
            ctx: ThreadContext::default(),
            state: State::Running,
        };
        let mut threads = vec![base_thread];
        let mut available_threads: Vec<Thread> = (1..MAX_THREADS).map(Thread::new).collect();
        threads.append(&mut available_threads);
        Self {
            threads,
            current: 0,
        }
    }
    pub fn from_addr<'a>(addr: usize) -> &'a mut Self {
        unsafe {
            let ptr = addr as *mut Self;
            &mut *ptr
        }
    }
}

fn main() {
    println!("Hello, world!");
    let mut rt = Runtime::new();

    unsafe {
        RUNTIME_ADDR = rt.get_addr();
    }
    rt.spawn(|| {
        let id = 1;
        for i in 1..10 {
            println!("thread: {}, counter: {}", id, i);
            let rt = Runtime::from_addr(unsafe { RUNTIME_ADDR });
            rt.yield_thread();
        }
        println!("1 done");
    });
    rt.spawn(|| {
        let id = 2;
        for i in 1..10 {
            println!("thread: {}, counter: {}", id, i);
            let rt = Runtime::from_addr(unsafe { RUNTIME_ADDR });
            rt.yield_thread();
        }
        println!("2 done");
    });
    rt.run();
}
