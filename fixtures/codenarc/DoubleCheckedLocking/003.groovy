
            if (!object) {
                synchronized(this) {
                    if (!object) {
                        object = createObject()
                    }
                }
            }
        