
            class EventListener {
                EventListener(EventPublisher publisher) {
                    publisher.register(this)
                    new WorkThread(publisher, this).start()
                    new AnotherWorkThread(listener: this).start()
                }
            }
        