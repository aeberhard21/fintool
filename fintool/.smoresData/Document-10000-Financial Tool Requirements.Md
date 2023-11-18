# User Requirements Specification - Financial Tool Requirements
# Introduction    
## Purpose    
This document defines the system requirements for a financial planner tool. This tool is intended to be a one-stop shop that allows a user, or users, to track multiple facets of their financial portfolio. Herein, this financial tool shall be referred to as the name "fintool."    

## Scope    
The scope of this document is the fintool software system.    

## Intended audience    
This document is intended for developers of the fintool software system.    

## References    
| ID | Reference Name | Title          |
|----|----------------|----------------|
| [1]| doc1           | Document title |    

## Glossary    
Defined terms are capitalized in this document. These terms are defined in the table below.    

| Term | Definition             |
|------|------------------------|
| TERM | The definition for TERM|    

# Overall description    
Fintool is intended to be a tool that integrates many available products into a single product. Namely, this includes a way for a user to set and manage a budget, balance their checkbook, estimate taxes owed, track growth within investment portfolios and summarize ther net worth.    

# User Requirements    
The following requirements will establish the data managed by the tool and its user interface.    

| _Requirement 11800_         | Fintool shall be written in the Rust programming language.                                                                                                       |    
|-----------------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------|    
| ___Translation rationale___ | __Rust is a strongly typed compiled language that provides features for safe memory access and class-like definition of implementations and associated traits.__ |    

| _Requirement 11900_         | Fintool shall define an implementation of a budget.<br/>                                                                                                                                                                                                                                            |    
|-----------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|    
| ___Translation rationale___ | __A budget is a useful tool for creating a planned set of expenditure amongst a list of expense categories for a user. It allows a user to work towards a goal of spending a certain amount in a given period of time and provides the user guidelines depicting how they can spend their income.__ |    

| _Requirement 12000_         | New functional requirement. |    
|-----------------------------|-----------------------------|    
| ___Translation rationale___ | __undefined__               |    

