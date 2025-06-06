use crate::logging::{logger::LogSink, LogEvent};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::interval;

/// Buffered sink that batches log events before forwarding to another sink
pub struct BufferedSink {
    _buffer: Arc<Mutex<VecDeque<LogEvent>>>,
    inner_sink: Arc<dyn LogSink>,
    _max_buffer_size: usize,
    _flush_interval: Duration,
    tx: mpsc::Sender<BufferCommand>,
}

enum BufferCommand {
    Log(LogEvent),
    Flush,
    Shutdown,
}

impl BufferedSink {
    pub fn new(
        inner_sink: Arc<dyn LogSink>,
        max_buffer_size: usize,
        flush_interval: Duration,
    ) -> Self {
        let buffer = Arc::new(Mutex::new(VecDeque::with_capacity(max_buffer_size)));
        let (tx, mut rx) = mpsc::channel::<BufferCommand>(1000);

        let buffer_clone = buffer.clone();
        let inner_sink_clone = inner_sink.clone();

        // Spawn background task for periodic flushing
        tokio::spawn(async move {
            let mut flush_timer = interval(flush_interval);

            loop {
                tokio::select! {
                    _ = flush_timer.tick() => {
                        Self::flush_buffer(&buffer_clone, &inner_sink_clone);
                    }
                    Some(cmd) = rx.recv() => {
                        match cmd {
                            BufferCommand::Log(event) => {
                                let should_flush = {
                                    let mut buffer = buffer_clone.lock().unwrap();
                                    buffer.push_back(event);
                                    buffer.len() >= max_buffer_size
                                };

                                if should_flush {
                                    Self::flush_buffer(&buffer_clone, &inner_sink_clone);
                                }
                            }
                            BufferCommand::Flush => {
                                Self::flush_buffer(&buffer_clone, &inner_sink_clone);
                            }
                            BufferCommand::Shutdown => {
                                Self::flush_buffer(&buffer_clone, &inner_sink_clone);
                                break;
                            }
                        }
                    }
                }
            }
        });

        Self {
            _buffer: buffer,
            inner_sink,
            _max_buffer_size: max_buffer_size,
            _flush_interval: flush_interval,
            tx,
        }
    }

    fn flush_buffer(buffer: &Arc<Mutex<VecDeque<LogEvent>>>, sink: &Arc<dyn LogSink>) {
        let events: Vec<LogEvent> = {
            let mut buffer = buffer.lock().unwrap();
            buffer.drain(..).collect()
        };

        for event in events {
            sink.log(event);
        }

        sink.flush();
    }
}

impl LogSink for BufferedSink {
    fn log(&self, event: LogEvent) {
        // Try to send to background task, fall back to direct write if channel is full
        if let Err(mpsc::error::TrySendError::Full(_)) =
            self.tx.try_send(BufferCommand::Log(event.clone()))
        {
            // Channel full, write directly
            self.inner_sink.log(event);
        }
    }

    fn flush(&self) {
        // Send flush command to background task
        let _ = self.tx.try_send(BufferCommand::Flush);
    }
}

impl Drop for BufferedSink {
    fn drop(&mut self) {
        // Send shutdown command to background task
        let _ = self.tx.try_send(BufferCommand::Shutdown);
    }
}
