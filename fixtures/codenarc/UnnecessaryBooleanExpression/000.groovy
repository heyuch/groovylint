
            def ready = value && true
            if (value || true) {
                println 'ok'
            }
            def result = value && false

            ready = true && value
            if (true || value) { println 'ok' }
            result = false && value
        