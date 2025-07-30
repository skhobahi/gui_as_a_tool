#!/usr/bin/env node

const WebSocket = require('ws');
const { v4: uuidv4 } = require('uuid');

class AgentHUDWebSocketServer {
    constructor() {
        this.agents = new Map();
        this.humanRequests = new Map();
        this.guiClients = new Set();
        this.port = 8080;
        
        console.log('🚀 Starting Agent HUD v5 WebSocket Server...');
        this.start();
    }
    
    start() {
        this.wss = new WebSocket.Server({ port: this.port });
        
        this.wss.on('connection', (ws) => {
            console.log('🔌 New WebSocket connection');
            
            ws.on('message', (data) => {
                try {
                    const message = JSON.parse(data.toString());
                    this.handleMessage(ws, message);
                } catch (error) {
                    console.error('❌ Failed to parse message:', error);
                }
            });
            
            ws.on('close', () => {
                console.log('📴 WebSocket connection closed');
                this.handleDisconnection(ws);
            });
            
            ws.on('error', (error) => {
                console.error('❌ WebSocket error:', error);
            });
        });
        
        console.log(`✅ WebSocket server running on port ${this.port}`);
        console.log(`📱 GUI can connect at: ws://127.0.0.1:${this.port}`);
        
        // Create some demo data
        this.createDemoData();
    }
    
    handleMessage(ws, message) {
        console.log('📨 Received message:', message);
        
        switch (message.type) {
            case 'register-gui':
                this.registerGUI(ws);
                break;
            case 'register-agent':
                this.registerAgent(ws, message.data);
                break;
            case 'human-input-request':
                this.handleHumanInputRequest(ws, message.data);
                break;
            case 'human-input-response':
                this.handleHumanInputResponse(message.requestId, message.response);
                break;
            case 'agent-update':
                this.handleAgentUpdate(ws, message.data);
                break;
            default:
                console.log('❓ Unknown message type:', message.type);
        }
    }
    
    registerGUI(ws) {
        console.log('🖥️ GUI client registered');
        ws.clientType = 'gui';
        this.guiClients.add(ws);
        
        // Send current agents to new GUI client
        for (const [id, agent] of this.agents) {
            this.sendToClient(ws, {
                type: 'agent-connected',
                data: agent
            });
        }
        
        // Send current requests to new GUI client
        for (const [id, request] of this.humanRequests) {
            this.sendToClient(ws, {
                type: 'human-input-request',
                data: request
            });
        }
    }
    
    registerAgent(ws, agentData) {
        const agentId = uuidv4();
        const agent = {
            id: agentId,
            name: agentData.name || 'Unknown Agent',
            status: 'Connected',
            timestamp: new Date().toISOString()
        };
        
        console.log('🤖 Agent registered:', agent.name);
        ws.clientType = 'agent';
        ws.agentId = agentId;
        this.agents.set(agentId, agent);
        
        // Notify GUI clients
        this.broadcastToGUIs({
            type: 'agent-connected',
            data: agent
        });
    }
    
    handleHumanInputRequest(ws, requestData) {
        const requestId = uuidv4();
        const agent = this.agents.get(ws.agentId);
        
        const request = {
            id: requestId,
            agent_id: ws.agentId,
            agent_name: agent ? agent.name : 'Unknown Agent',
            request_type: requestData.request_type || 'input',
            message: requestData.message || 'Human input required',
            priority: requestData.priority || 'Medium',
            status: 'Pending',
            timestamp: new Date().toISOString(),
            options: requestData.options || []
        };
        
        console.log('❓ Human input request:', request.message);
        this.humanRequests.set(requestId, request);
        
        // Store the agent's WebSocket for response
        request.agentWs = ws;
        
        // Notify GUI clients
        this.broadcastToGUIs({
            type: 'human-input-request',
            data: request
        });
    }
    
    handleHumanInputResponse(requestId, response) {
        const request = this.humanRequests.get(requestId);
        if (!request) {
            console.error('❌ Request not found:', requestId);
            return;
        }
        
        console.log('✅ Human input response:', response);
        request.status = 'Completed';
        request.response = response;
        
        // Send response back to agent
        if (request.agentWs && request.agentWs.readyState === WebSocket.OPEN) {
            this.sendToClient(request.agentWs, {
                type: 'human-input-response',
                requestId: requestId,
                response: response
            });
        }
        
        // Update GUI clients
        this.broadcastToGUIs({
            type: 'human-input-request',
            data: request
        });
    }
    
    handleAgentUpdate(ws, updateData) {
        const agent = this.agents.get(ws.agentId);
        if (agent) {
            Object.assign(agent, updateData);
            this.broadcastToGUIs({
                type: 'agent-update',
                data: agent
            });
        }
    }
    
    handleDisconnection(ws) {
        if (ws.clientType === 'gui') {
            this.guiClients.delete(ws);
        } else if (ws.clientType === 'agent' && ws.agentId) {
            const agent = this.agents.get(ws.agentId);
            if (agent) {
                console.log('🔌 Agent disconnected:', agent.name);
                this.agents.delete(ws.agentId);
                
                // Notify GUI clients
                this.broadcastToGUIs({
                    type: 'agent-disconnected',
                    data: { agentId: ws.agentId }
                });
            }
        }
    }
    
    sendToClient(ws, message) {
        if (ws.readyState === WebSocket.OPEN) {
            ws.send(JSON.stringify(message));
        }
    }
    
    broadcastToGUIs(message) {
        for (const gui of this.guiClients) {
            this.sendToClient(gui, message);
        }
    }
    
    createDemoData() {
        // Create a demo agent after a short delay
        setTimeout(() => {
            const demoAgent = {
                id: 'demo-agent-001',
                name: 'Demo Analysis Agent',
                status: 'Active',
                timestamp: new Date().toISOString()
            };
            
            this.agents.set(demoAgent.id, demoAgent);
            console.log('🎯 Created demo agent:', demoAgent.name);
            
            // Create a demo human input request
            setTimeout(() => {
                const demoRequest = {
                    id: 'demo-request-001',
                    agent_id: demoAgent.id,
                    agent_name: demoAgent.name,
                    request_type: 'approval',
                    message: 'Should I proceed with the data analysis? This will process 10,000 records.',
                    priority: 'High',
                    status: 'Pending',
                    timestamp: new Date().toISOString(),
                    options: ['Yes, proceed', 'No, cancel', 'Review settings first']
                };
                
                this.humanRequests.set(demoRequest.id, demoRequest);
                console.log('📝 Created demo request:', demoRequest.message);
                
                // Broadcast to any connected GUIs
                this.broadcastToGUIs({
                    type: 'agent-connected',
                    data: demoAgent
                });
                
                this.broadcastToGUIs({
                    type: 'human-input-request',
                    data: demoRequest
                });
                
            }, 2000);
        }, 1000);
    }
}

// Check if required modules are available
try {
    require('ws');
    require('uuid');
} catch (error) {
    console.error('❌ Missing required Node.js modules. Please install:');
    console.error('npm install ws uuid');
    process.exit(1);
}

// Start the server
new AgentHUDWebSocketServer();