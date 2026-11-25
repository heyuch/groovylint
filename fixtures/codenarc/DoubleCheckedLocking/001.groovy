
            if (object == null) {
                synchronized(this) {
                    if (object == null) {
                        object = createObject()
                    }
                }
            }
        