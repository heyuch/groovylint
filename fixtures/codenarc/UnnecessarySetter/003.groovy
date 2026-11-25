
            void doStuff() {
                x.setProperty(1)
                x.setProperty(this.getA())
                x.setOther(123)
                
                setOther(456)
            }
            
            int calculateStuff() {
                return setCount(123)        // No violation
            }
        