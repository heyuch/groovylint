
            class Person {
                String name
                List addresses

                synchronized void setAddresses(List addresses) {
                    this.addresses.clear()
                    this.addresses.addAll(addresses)
                }
            }