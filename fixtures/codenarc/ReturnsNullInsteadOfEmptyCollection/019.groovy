
            Closure c = {
                List things = this.stuff.collect { return [ theThing: it ] } //this return is detected (wrongly)
                if (things.isEmpty()) {
                    return //violation is raised here
                }
                println things
            }
        