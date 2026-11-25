

                // 'this' reference ignored
                firstName = 'Hamlet'
                lastName = "D'Arcy"
                employer = 'Canoo'
                street = 'Kirschgaraten 5'
                city = 'Basel'
                zipCode = '4051'
                this.firstName = 'Hamlet'
                this.lastName = "D'Arcy"
                this.employer = 'Canoo'
                this.street = 'Kirschgaraten 5'
                this.city = 'Basel'
                this.zipCode = '4051'
                method1()
                method2()
                method4()
                method6()
                method5()
                this.method1()
                this.method2()
                this.method4()
                this.method6()
                this.method5()

                method1()
                // only 5 references
                def p = new Person()
                p.firstName = 'Hamlet'
                p.lastName = "D'Arcy"
                p.employer = 'Canoo'
                p.street = 'Kirschgaraten 5'
                p.city = 'Basel'

                def p1 = new Person().with {
                    firstName = 'Hamlet'
                    lastName = "D'Arcy"
                    employer = 'Canoo'
                    street = 'Kirschgaraten 5'
                    city = 'Basel'
                    zipCode = '4051'
                }

                def p2 = new Person().identity {
                    firstName = 'Hamlet'
                    lastName = "D'Arcy"
                    employer = 'Canoo'
                    street = 'Kirschgaraten 5'
                    city = 'Basel'
                    zipCode = '4051'
                } 