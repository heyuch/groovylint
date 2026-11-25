
            package org.grails.datastore.gorm

            trait GormEntity<D> {
                static <D> List<D> foobar() {
                    Collections.emptyList()
                }
            }

            class SomeEntity implements GormEntity<SomeEntity> { }

            class SomeService {

                void someMethod() {
                    SomeEntity.foobar()
                }

            }
        