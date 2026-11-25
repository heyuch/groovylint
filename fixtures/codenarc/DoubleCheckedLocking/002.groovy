
            if (null == object) {
                synchronized(this) {
                    if (null == object) {
                        object = createObject()
                    }
                }
            }
        